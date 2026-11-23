//! LEMU Plum-Flat Virtual CPU with writable memory

#[allow(unused_imports)]
use alloc::format;

use ir86::encoding::sib::SibIndex;

use crate::_prelude_::*;
use crate::MANUFACTURER_STRING;
use crate::alu::Alu;
use crate::cpu::{ProcessorState, RtReg};
use crate::flat::tracer::TraceDecoder;
use crate::flat::uop::{
    AddrIndex, BinaryRmwKind, BitAction, LoopKind, ShiftRmwOp, UnaryRmwOp, Uop, UopMinor,
};
use crate::gpr::PartialRegister;

#[cfg(feature = "flag-elision")]
#[path = "../elide_uop.rs"]
mod elide_uop;
#[cfg(feature = "x87")]
mod fpu;
mod memory;
#[cfg(feature = "uop-profile")]
pub mod profile;
mod string;
pub mod tracer;
pub mod uop;
pub use memory::InitError;

/// LEMU Plum-Flat Virtual CPU.
pub struct FlatUME {
    #[cfg(feature = "uop-profile")]
    profile: profile::ExecutionProfile,
    memory: Vec<u8>,
    base: u32,
    flat: memory::Flat,
    state: ProcessorState,
    tracer: TraceDecoder,
}

impl Emulator for FlatUME {
    fn state(&self) -> &ProcessorState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut ProcessorState {
        &mut self.state
    }

    fn execute(&mut self) -> Result<(), Exception> {
        #[cfg(feature = "uop-profile")]
        if let Some(eip) = self.profile.retry_pending.take() {
            self.profile.counters.retries += u64::from(self.flat.eip == eip);
        }
        // The host owns segment state between runs; GS-relative uops read this mirror.
        let gs_base = self.state.sr(SrIndex::GS).base.0;
        self.state.reg(RtReg::GsBase).e().write(gs_base);
        self.prepare_flat();
        self._execute().map_err(|err| {
            self.stop_flat();
            #[cfg(feature = "uop-profile")]
            if !matches!(err, Exception::Swi(_)) {
                self.profile.counters.faults += 1;
                self.profile.retry_pending = Some(Offset32(self.state.eip().read()));
            }
            err
        })
    }

    fn resume_next(&mut self) {
        if let Some((_, next)) = self.flat.stopped.take() {
            self.flat.eip = next;
            self.tracer.set_current_upc(AddrIndex::BAD_ADDRESS);
        }
    }

    fn is_flat(&self) -> bool {
        true
    }

    fn to_semi_flat(&mut self) -> Result<FlatUME, Exception> {
        Err(Exception::OutOfCode)
    }

    fn memory(&self) -> (&[u8], Offset32) {
        (&self.memory, Offset32(self.base))
    }

    fn brk(&mut self, address: u32) -> Result<u32, u32> {
        // A full 32-bit interval has an exclusive end represented as zero.
        let end = u64::from(self.base) + self.memory.len() as u64;
        let current = end as u32;
        if address == 0 {
            return Ok(current);
        }
        if u64::from(address) < end {
            return Err(current);
        }
        let len = (address - self.base) as usize;
        self.grow_memory(len).map_err(|()| current)?;
        Ok(address)
    }

    fn write_memory(&mut self, address: u32, bytes: &[u8]) -> Result<(), Exception> {
        self.write_bytes(address, bytes)
    }

    fn fill_memory(&mut self, address: u32, value: u8, size: usize) -> Result<(), Exception> {
        self.fill_bytes(address, value, size)
    }
}

impl FlatUME {
    #[rustfmt::skip]
    const BRAND_STRING: &[u8; 48] = &nul_padded("LEMU Plum-Flat Virtual CPU @ 42MHz");

    /// Creates a new instance
    pub(crate) fn new(
        memory: Vec<u8>,
        guest_base: Offset32,
        entry: Offset32,
        stack_pointer: Offset32,
    ) -> Result<Self, InitError> {
        let end = u64::from(guest_base.0)
            .checked_add(memory.len() as u64)
            .filter(|end| *end <= 1u64 << 32)
            .ok_or(InitError::AddressOverflow)?;
        if entry.0 < guest_base.0 || u64::from(entry.0) >= end {
            return Err(InitError::EntryOutsideMemory);
        }
        // An empty descending stack may point immediately past the interval.
        if stack_pointer.0 < guest_base.0 || u64::from(stack_pointer.0) > end {
            return Err(InitError::StackOutsideMemory);
        }

        let state =
            ProcessorState::new(Generation::LATEST, MANUFACTURER_STRING, Self::BRAND_STRING);
        state.esp().write(stack_pointer.0);

        Ok(Self {
            #[cfg(feature = "uop-profile")]
            profile: profile::ExecutionProfile::default(),
            flat: memory::Flat::new(memory.len(), entry),
            memory,
            base: guest_base.0,
            state,
            tracer: TraceDecoder::new(guest_base),
        })
    }

    /// Pushes a value onto the stack
    #[inline]
    pub fn push(&mut self, value: u32) -> Option<()> {
        let new_esp = self.state.esp().read().wrapping_sub(4);
        self.write_memory32(new_esp, value).ok()?;
        self.state.esp().write(new_esp);
        Some(())
    }

    /// Pushes all general-purpose registers onto the stack
    #[inline]
    pub fn pushad(&mut self) -> Option<()> {
        let esp_temp = self.state.esp().read();
        let esp_bottom = esp_temp.wrapping_sub(4 * 8);
        let values = [
            self.state.edi().read(),
            self.state.esi().read(),
            self.state.ebp().read(),
            esp_temp,
            self.state.ebx().read(),
            self.state.edx().read(),
            self.state.ecx().read(),
            self.state.eax().read(),
        ];
        let mut bytes = [0; 32];
        for (chunk, value) in bytes.chunks_exact_mut(4).zip(values) {
            chunk.copy_from_slice(&value.to_le_bytes());
        }
        self.write_bytes(esp_bottom, &bytes).ok()?;
        self.state.esp().write(esp_bottom);
        Some(())
    }

    /// Pops a value from the stack
    #[inline]
    pub fn pop(&mut self) -> Option<u32> {
        let esp = self.state.esp().read();
        let new_esp = esp.wrapping_add(4);
        let value = self.read_memory32(esp).ok()?;
        self.state.esp().write(new_esp);
        Some(value)
    }

    /// Pops all general-purpose registers from the stack
    #[inline]
    pub fn popad(&mut self) -> Option<()> {
        let esp_temp = self.state.esp().read();
        let range = self.memory_range(esp_temp, 32).ok()?;
        let p = &self.memory()[range];
        let mut values = [0u32; 8];
        for (value, chunk) in values.iter_mut().zip(p.chunks_exact(4)) {
            *value = u32::from_le_bytes(chunk.try_into().unwrap());
        }
        self.state.edi().write(values[0]);
        self.state.esi().write(values[1]);
        self.state.ebp().write(values[2]);
        self.state.ebx().write(values[4]);
        self.state.edx().write(values[5]);
        self.state.ecx().write(values[6]);
        self.state.eax().write(values[7]);
        self.state.esp().modify(|esp| esp.wrapping_add(4 * 8));
        Some(())
    }

    #[inline(never)]
    fn pop_memory_destination(
        &mut self,
        base: Option<RtReg>,
        sib: Option<SibIndex>,
        disp: u32,
    ) -> Result<(), Exception> {
        let esp = self.state.esp().read();
        let value = self.read_memory32(esp).map_err(|_| Exception::StackFault)?;
        let next = esp.wrapping_add(4);
        let reg = |r: RtReg| {
            if r == RtReg::ESP {
                next
            } else {
                self.state.reg(r).e().read()
            }
        };
        let addr = if let Some(sib) = sib {
            let (base, index, scale) = sib.to_sib();
            base.map(|r| reg(r.into()))
                .unwrap_or(0)
                .wrapping_add(reg(index.into()).wrapping_shl(scale.shift()))
        } else {
            reg(base.unwrap())
        }
        .wrapping_add(disp);
        // Validate/store the destination before committing ESP. Memory can alias
        // the stack, and its effective address uses the incremented ESP.
        self.write_memory32(addr, value)?;
        self.state.esp().write(next);
        Ok(())
    }

    #[inline(never)]
    fn enter_zero(&mut self, bytes: u16) -> Result<(), Exception> {
        // TODO: lexical nesting >= 1 and 16-bit decoding are unsupported.
        let frame = self.state.esp().read().wrapping_sub(4);
        let next = frame
            .checked_sub(bytes as u32)
            .ok_or(Exception::StackFault)?;
        let value = self.state.ebp().read();
        self.write_memory32(frame, value)
            .map_err(|_| Exception::StackFault)?;
        self.state.ebp().write(frame);
        self.state.esp().write(next);
        Ok(())
    }

    #[inline]
    pub fn tracer(&self) -> &TraceDecoder {
        &self.tracer
    }

    /// Updates the EIP register from the current instruction pointer
    ///
    /// # Note
    /// This function may take O(n) time in the worst case.
    pub fn reflect_eip(&mut self) {
        let eip = self
            .tracer
            .instruction_at(self.tracer.current_upc())
            .map_or(self.flat.eip, |i| i.source);
        self.state.eip().write(eip.0);
    }

    /// Adjusts the state after an exception occurs
    #[inline]
    pub fn adjust_after_exception(&mut self) {
        self.reflect_eip();
        self.state.flags_mut().resolve();
    }

    #[inline]
    pub fn alu<'a>(&'a mut self) -> Alu<'a> {
        Alu::new(&mut self.state)
    }

    /// Resolves the effective address from a SIB index
    #[inline]
    pub fn resolve_sib(&self, sib: SibIndex) -> u32 {
        let (base, index, scale) = sib.to_sib();
        let base = base
            .map(|base| self.state.reg(base.into()).e().read())
            .unwrap_or(0);
        let index = self
            .state
            .reg(index.into())
            .e()
            .read()
            .wrapping_shl(scale.shift());
        base.wrapping_add(index)
    }

    #[inline]
    pub fn read_memory8(&self, addr: u32) -> Result<u8, Exception> {
        let range = self.memory_range(addr, 1)?;
        Ok(self.memory()[range.start])
    }

    #[inline]
    pub fn write_memory8(&mut self, addr: u32, value: u8) -> Result<(), Exception> {
        self.write_bytes(addr, &value.to_le_bytes())
    }

    #[inline]
    pub fn read_memory16(&self, addr: u32) -> Result<u16, Exception> {
        let range = self.memory_range(addr, 2)?;
        Ok(u16::from_le_bytes(self.memory()[range].try_into().unwrap()))
    }

    #[inline]
    pub fn write_memory16(&mut self, addr: u32, value: u16) -> Result<(), Exception> {
        self.write_bytes(addr, &value.to_le_bytes())
    }

    #[inline]
    pub fn read_memory32(&self, addr: u32) -> Result<u32, Exception> {
        let range = self.memory_range(addr, 4)?;
        Ok(u32::from_le_bytes(self.memory()[range].try_into().unwrap()))
    }

    #[inline]
    pub fn write_memory32(&mut self, addr: u32, value: u32) -> Result<(), Exception> {
        self.write_bytes(addr, &value.to_le_bytes())
    }

    #[inline]
    fn popcount_flags(&mut self, zero: bool) {
        self.state.flags_mut().clear(Flags::from_bits(0x8d5));
        self.state.flags_mut().set_dynamic(Flags::ZF, zero);
    }

    /// Inputs are captured before any partial-register write, including aliases.
    fn bit_operation(&mut self, action: BitAction, rd: RtReg, index: u32, narrow: bool) {
        let value = self.state.reg(rd).e().read();
        let mask = 1u32 << (index & if narrow { 15 } else { 31 });
        let result = match action {
            BitAction::Test => value,
            BitAction::Set => value | mask,
            BitAction::Reset => value & !mask,
            BitAction::Complement => value ^ mask,
        };
        // Resolve old CF-dependent lazy flags before replacing CF alone.
        self.state.flags_mut().resolve();
        self.state
            .flags_mut()
            .set_dynamic(Flags::CF, value & mask != 0);
        if action != BitAction::Test {
            if narrow {
                self.state.reg(rd).w().write(result as u16);
            } else {
                self.state.reg(rd).e().write(result);
            }
        }
    }

    /// A whole-word read validates the subsequent store in private data memory.
    /// Capture the adjusted address before resolving flags; no guest register changes.
    // Outlining preserves the common Wasm loop; native RET favors inlining.
    // See the frequency review results for the target-specific comparison.
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    #[cfg_attr(not(target_arch = "wasm32"), inline(always))]
    fn bit_memory_update(
        &mut self,
        action: BitAction,
        addr: u32,
        index: u32,
        narrow: bool,
    ) -> Result<(), Exception> {
        let value = if narrow {
            self.read_memory16(addr)? as u32
        } else {
            self.read_memory32(addr)?
        };
        let mask = 1u32 << (index & if narrow { 15 } else { 31 });
        let result = match action {
            BitAction::Set => value | mask,
            BitAction::Reset => value & !mask,
            BitAction::Complement => value ^ mask,
            BitAction::Test => unreachable!("read-only BT uses the existing load/bit path"),
        };
        if narrow {
            self.write_memory16(addr, result as u16)?;
        } else {
            self.write_memory32(addr, result)?;
        }
        self.state.flags_mut().resolve();
        self.state
            .flags_mut()
            .set_dynamic(Flags::CF, value & mask != 0);
        Ok(())
    }

    /// Keep the width/operation dispatch out of the ordinary CPU loop.
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    #[cfg_attr(not(target_arch = "wasm32"), inline(always))]
    fn shift_memory_update(
        &mut self,
        op: ShiftRmwOp,
        addr: u32,
        count: u8,
    ) -> Result<(), Exception> {
        match op {
            ShiftRmwOp::Rol => {
                let dst = self.read_memory32(addr)?;
                let result = self.alu().rol32(dst, count);
                self.write_memory32(addr, result)?;
            }
            ShiftRmwOp::Rol16 => {
                let dst = self.read_memory16(addr)?;
                let result = self.alu().rol16(dst, count);
                self.write_memory16(addr, result)?;
            }
            ShiftRmwOp::Rol8 => {
                let dst = self.read_memory8(addr)?;
                let result = self.alu().rol8(dst, count);
                self.write_memory8(addr, result)?;
            }
            ShiftRmwOp::Ror => {
                let dst = self.read_memory32(addr)?;
                let result = self.alu().ror32(dst, count);
                self.write_memory32(addr, result)?;
            }
            ShiftRmwOp::Ror16 => {
                let dst = self.read_memory16(addr)?;
                let result = self.alu().ror16(dst, count);
                self.write_memory16(addr, result)?;
            }
            ShiftRmwOp::Ror8 => {
                let dst = self.read_memory8(addr)?;
                let result = self.alu().ror8(dst, count);
                self.write_memory8(addr, result)?;
            }
            ShiftRmwOp::Sar => {
                let dst = self.read_memory32(addr)?;
                let result = self.alu().sar32(dst, count);
                self.write_memory32(addr, result)?;
            }
            ShiftRmwOp::Sar16 => {
                let dst = self.read_memory16(addr)?;
                let result = self.alu().sar16(dst, count);
                self.write_memory16(addr, result)?;
            }
            ShiftRmwOp::Sar8 => {
                let dst = self.read_memory8(addr)?;
                let result = self.alu().sar8(dst, count);
                self.write_memory8(addr, result)?;
            }
            ShiftRmwOp::Shl => {
                let dst = self.read_memory32(addr)?;
                let result = self.alu().shl32(dst, count);
                self.write_memory32(addr, result)?;
            }
            ShiftRmwOp::Shl16 => {
                let dst = self.read_memory16(addr)?;
                let result = self.alu().shl16(dst, count);
                self.write_memory16(addr, result)?;
            }
            ShiftRmwOp::Shl8 => {
                let dst = self.read_memory8(addr)?;
                let result = self.alu().shl8(dst, count);
                self.write_memory8(addr, result)?;
            }
            ShiftRmwOp::Shr => {
                let dst = self.read_memory32(addr)?;
                let result = self.alu().shr32(dst, count);
                self.write_memory32(addr, result)?;
            }
            ShiftRmwOp::Shr16 => {
                let dst = self.read_memory16(addr)?;
                let result = self.alu().shr16(dst, count);
                self.write_memory16(addr, result)?;
            }
            ShiftRmwOp::Shr8 => {
                let dst = self.read_memory8(addr)?;
                let result = self.alu().shr8(dst, count);
                self.write_memory8(addr, result)?;
            }
        }
        Ok(())
    }

    /// Outline unary memory updates so their width/operation cases do not expand
    /// the common CPU dispatch table or duplicate its register operand extraction.
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    #[cfg_attr(not(target_arch = "wasm32"), inline(always))]
    fn unary_memory_update(&mut self, op: UnaryRmwOp, addr: u32) -> Result<(), Exception> {
        match op {
            UnaryRmwOp::Dec => {
                let dst = self.read_memory32(addr)?;
                let result = self.alu().dec32(dst);
                self.write_memory32(addr, result)?;
            }
            UnaryRmwOp::Dec16 => {
                let dst = self.read_memory16(addr)?;
                let result = self.alu().dec16(dst);
                self.write_memory16(addr, result)?;
            }
            UnaryRmwOp::Dec8 => {
                let dst = self.read_memory8(addr)?;
                let result = self.alu().dec8(dst);
                self.write_memory8(addr, result)?;
            }
            UnaryRmwOp::Inc => {
                let dst = self.read_memory32(addr)?;
                let result = self.alu().inc32(dst);
                self.write_memory32(addr, result)?;
            }
            UnaryRmwOp::Inc16 => {
                let dst = self.read_memory16(addr)?;
                let result = self.alu().inc16(dst);
                self.write_memory16(addr, result)?;
            }
            UnaryRmwOp::Inc8 => {
                let dst = self.read_memory8(addr)?;
                let result = self.alu().inc8(dst);
                self.write_memory8(addr, result)?;
            }
            UnaryRmwOp::Neg => {
                let dst = self.read_memory32(addr)?;
                let result = self.alu().sub32(0, dst);
                self.write_memory32(addr, result)?;
            }
            UnaryRmwOp::Neg16 => {
                let dst = self.read_memory16(addr)?;
                let result = self.alu().sub16(0, dst);
                self.write_memory16(addr, result)?;
            }
            UnaryRmwOp::Neg8 => {
                let dst = self.read_memory8(addr)?;
                let result = self.alu().sub8(0, dst);
                self.write_memory8(addr, result)?;
            }
            UnaryRmwOp::Not => {
                let dst = self.read_memory32(addr)?;
                let result = !dst;
                self.write_memory32(addr, result)?;
            }
            UnaryRmwOp::Not16 => {
                let dst = self.read_memory16(addr)?;
                let result = !dst;
                self.write_memory16(addr, result)?;
            }
            UnaryRmwOp::Not8 => {
                let dst = self.read_memory8(addr)?;
                let result = !dst;
                self.write_memory8(addr, result)?;
            }
        }
        Ok(())
    }

    #[cfg(feature = "flag-elision")]
    fn value_update32(&mut self, op: BinaryRmwKind, rd: RtReg, rhs: u32) {
        let lhs = self.state.reg(rd).e().read();
        let value = match op {
            BinaryRmwKind::Add => lhs.wrapping_add(rhs),
            BinaryRmwKind::Sub => lhs.wrapping_sub(rhs),
            BinaryRmwKind::And => lhs & rhs,
            BinaryRmwKind::Or => lhs | rhs,
            BinaryRmwKind::Xor => lhs ^ rhs,
            BinaryRmwKind::Adc => lhs
                .wrapping_add(rhs)
                .wrapping_add(u32::from(self.state.flags_mut().recompute_cf())),
            BinaryRmwKind::Sbb => lhs
                .wrapping_sub(rhs)
                .wrapping_sub(u32::from(self.state.flags_mut().recompute_cf())),
        };
        self.state.reg(rd).e().write(value);
    }

    // Supplemental binary RMW operations share width-specific helpers. This keeps
    // the primary dispatch table small without changing the established fast cases.
    #[cfg_attr(target_arch = "wasm32", inline(never))]
    #[cfg_attr(not(target_arch = "wasm32"), inline(always))]
    fn binary_memory_update32(
        &mut self,
        kind: BinaryRmwKind,
        addr: u32,
        src: u32,
    ) -> Result<(), Exception> {
        let dst = self.read_memory32(addr)?;
        let result = match kind {
            BinaryRmwKind::Adc => self.alu().adc32(dst, src),
            BinaryRmwKind::Add => self.alu().add32(dst, src),
            BinaryRmwKind::And => self.alu().and32(dst, src),
            BinaryRmwKind::Or => self.alu().or32(dst, src),
            BinaryRmwKind::Sbb => self.alu().sbb32(dst, src),
            BinaryRmwKind::Sub => self.alu().sub32(dst, src),
            BinaryRmwKind::Xor => self.alu().xor32(dst, src),
        };
        self.write_memory32(addr, result)
    }

    #[cfg_attr(target_arch = "wasm32", inline(never))]
    #[cfg_attr(not(target_arch = "wasm32"), inline(always))]
    fn binary_memory_update16(
        &mut self,
        kind: BinaryRmwKind,
        addr: u32,
        src: u16,
    ) -> Result<(), Exception> {
        let dst = self.read_memory16(addr)?;
        let result = match kind {
            BinaryRmwKind::Adc => self.alu().adc16(dst, src),
            BinaryRmwKind::Add => self.alu().add16(dst, src),
            BinaryRmwKind::And => self.alu().and16(dst, src),
            BinaryRmwKind::Or => self.alu().or16(dst, src),
            BinaryRmwKind::Sbb => self.alu().sbb16(dst, src),
            BinaryRmwKind::Sub => self.alu().sub16(dst, src),
            BinaryRmwKind::Xor => self.alu().xor16(dst, src),
        };
        self.write_memory16(addr, result)
    }

    #[cfg_attr(target_arch = "wasm32", inline(never))]
    #[cfg_attr(not(target_arch = "wasm32"), inline(always))]
    fn binary_memory_update8(
        &mut self,
        kind: BinaryRmwKind,
        addr: u32,
        src: u8,
    ) -> Result<(), Exception> {
        let dst = self.read_memory8(addr)?;
        let result = match kind {
            BinaryRmwKind::Adc => self.alu().adc8(dst, src),
            BinaryRmwKind::Add => self.alu().add8(dst, src),
            BinaryRmwKind::And => self.alu().and8(dst, src),
            BinaryRmwKind::Or => self.alu().or8(dst, src),
            BinaryRmwKind::Sbb => self.alu().sbb8(dst, src),
            BinaryRmwKind::Sub => self.alu().sub8(dst, src),
            BinaryRmwKind::Xor => self.alu().xor8(dst, src),
        };
        self.write_memory8(addr, result)
    }

    /// Executes the emulation
    fn _execute(&mut self) -> Result<(), Exception> {
        loop {
            let uop = match self.fetch_cached() {
                Some(uop) => uop,
                None => {
                    return Err(self.flat.fetch_error.take().unwrap_or(Exception::OutOfCode));
                }
            };

            #[cfg(feature = "uop-profile")]
            if matches!(uop, Uop::CheckWrite(_)) {
                self.profile.record_internal();
            } else {
                self.profile.record(&self.tracer);
            }

            match uop {
                #[cfg(feature = "x87")]
                Uop::Fpu(op) => self.execute_fpu(op)?,
                #[cfg(feature = "flag-elision")]
                Uop::ValueAddR(rd, rs) => {
                    let dst = self.state.reg(rd).e();
                    dst.write(dst.read().wrapping_add(self.state.reg(rs).e().read()));
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueXorR(rd, rs) => {
                    let dst = self.state.reg(rd).e();
                    dst.write(dst.read() ^ self.state.reg(rs).e().read());
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueAndI(rd, imm) => {
                    let dst = self.state.reg(rd).e();
                    dst.write(dst.read() & imm);
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueAddI(rd, imm) => {
                    let dst = self.state.reg(rd).e();
                    dst.write(dst.read().wrapping_add(imm));
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueSubI(rd, imm) => {
                    let dst = self.state.reg(rd).e();
                    dst.write(dst.read().wrapping_sub(imm));
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueSubR(rd, rs) => {
                    let dst = self.state.reg(rd).e();
                    dst.write(dst.read().wrapping_sub(self.state.reg(rs).e().read()));
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueAndR(rd, rs) => {
                    let dst = self.state.reg(rd).e();
                    dst.write(dst.read() & self.state.reg(rs).e().read());
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueOrR(rd, rs) => {
                    let dst = self.state.reg(rd).e();
                    dst.write(dst.read() | self.state.reg(rs).e().read());
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueOrI(rd, imm) => {
                    let dst = self.state.reg(rd).e();
                    dst.write(dst.read() | imm);
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueXorI(rd, imm) => {
                    let dst = self.state.reg(rd).e();
                    dst.write(dst.read() ^ imm);
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueReg(op, rd, rs) => {
                    self.value_update32(op, rd, self.state.reg(rs).e().read())
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueImm(op, rd, v) => self.value_update32(op, rd, v),
                #[cfg(feature = "flag-elision")]
                Uop::ValueNeg(rd) => {
                    let dst = self.state.reg(rd).e();
                    dst.write(dst.read().wrapping_neg());
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueMulR(rd, rs) => {
                    let dst = self.state.reg(rd).e();
                    dst.write(dst.read().wrapping_mul(self.state.reg(rs).e().read()));
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueMulRI(rd, rs, imm) => {
                    self.state
                        .reg(rd)
                        .e()
                        .write(self.state.reg(rs).e().read().wrapping_mul(imm as u32));
                }
                #[cfg(feature = "flag-elision")]
                Uop::ValueShift(op, rd, count) => {
                    use uop::ValueShift::*;
                    let n = u32::from(count.unwrap_or_else(|| self.state.cl().read()));
                    let dst = self.state.reg(rd).e();
                    let value = dst.read();
                    dst.write(match op {
                        Shl => value.wrapping_shl(n),
                        Shr => value.wrapping_shr(n),
                        Sar => (value as i32).wrapping_shr(n) as u32,
                    });
                }
                Uop::AdcI(rd, imm) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().adc32(dst, imm);
                    self.state.reg(rd).e().write(result);
                }
                Uop::AdcI16(rd, imm) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().adc16(dst, imm);
                    self.state.reg(rd).w().write(result);
                }

                Uop::AdcI8(rd, imm) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().adc8(dst, imm);
                    self.state.reg8(rd).write(result);
                }

                Uop::AdcR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().adc32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::AdcR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().adc16(dst, src);
                    self.state.reg(rd).w().write(result);
                }
                Uop::AdcR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().adc8(dst, src);
                    self.state.reg8(rd).write(result);
                }

                Uop::AddI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().add32(dst, id);
                    self.state.reg(rd).e().write(result);
                }
                Uop::AddI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().add16(dst, iw);
                    self.state.reg(rd).w().write(result);
                }

                Uop::AddI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().add8(dst, ib);
                    self.state.reg8(rd).write(result);
                }

                Uop::AddR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().add32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::AddR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().add16(dst, src);
                    self.state.reg(rd).w().write(result);
                }
                Uop::AddR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().add8(dst, src);
                    self.state.reg8(rd).write(result);
                }
                Uop::AddRMW(rd, rs) => {
                    let addr = self.state.reg(rd).e().read();
                    let dst = self.read_memory32(addr)?;
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().add32(dst, src);
                    self.write_memory32(addr, result)?;
                }

                Uop::AddRMW8(rd, rs) => {
                    let addr = self.state.reg(rd).e().read();
                    let dst = self.read_memory8(addr)?;
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().add8(dst, src);
                    self.write_memory8(addr, result)?;
                }
                Uop::AndI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().and32(dst, id);
                    self.state.reg(rd).e().write(result);
                }
                Uop::AndI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().and16(dst, iw);
                    self.state.reg(rd).w().write(result);
                }

                Uop::AndI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().and8(dst, ib);
                    self.state.reg8(rd).write(result);
                }

                Uop::AndR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().and32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::AndR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().and16(dst, src);
                    self.state.reg(rd).w().write(result);
                }
                Uop::AndR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().and8(dst, src);
                    self.state.reg8(rd).write(result);
                }
                Uop::AndRMW(rd, rs) => {
                    let addr = self.state.reg(rd).e().read();
                    let dst = self.read_memory32(addr)?;
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().and32(dst, src);
                    self.write_memory32(addr, result)?;
                }
                Uop::BinaryIRMW(kind, addr, src) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = src;
                    self.binary_memory_update32(kind, addr, src)?;
                }
                Uop::BinaryIRMW16(kind, addr, src) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = src;
                    self.binary_memory_update16(kind, addr, src)?;
                }
                Uop::BinaryIRMW8(kind, addr, src) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = src;
                    self.binary_memory_update8(kind, addr, src)?;
                }

                Uop::BinaryRMW(kind, addr, src) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = self.state.reg(src).e().read();
                    self.binary_memory_update32(kind, addr, src)?;
                }
                Uop::BinaryRMW16(kind, addr, src) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = self.state.reg(src).w().read();
                    self.binary_memory_update16(kind, addr, src)?;
                }
                Uop::BinaryRMW8(kind, addr, src) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = self.state.reg8(src).read();
                    self.binary_memory_update8(kind, addr, src)?;
                }

                Uop::BitAddr(rs) | Uop::BitAddr16(rs) => {
                    let index = self.state.reg(rs).e().read();
                    let offset = if matches!(uop, Uop::BitAddr16(..)) {
                        ((index as i16 as i32) >> 4) << 1
                    } else {
                        ((index as i32) >> 5) << 2
                    };
                    let addr = self.state.reg(RtReg::MemAddr).e().read();
                    self.state
                        .reg(RtReg::MemAddr)
                        .e()
                        .write(addr.wrapping_add(offset as u32));
                }
                Uop::BitI(action, rd, index) | Uop::BitI16(action, rd, index) => {
                    self.bit_operation(action, rd, index as u32, matches!(uop, Uop::BitI16(..)));
                }
                Uop::BitIRMW(action, addr, index) => {
                    let addr = self.state.reg(addr).e().read();
                    let index = index as u32;

                    self.bit_memory_update(action, addr, index, false)?;
                }
                Uop::BitIRMW16(action, addr, index) => {
                    let addr = self.state.reg(addr).e().read();
                    let index = index as u32;

                    self.bit_memory_update(action, addr, index, true)?;
                }

                Uop::BitR(action, rd, rs) | Uop::BitR16(action, rd, rs) => {
                    let index = self.state.reg(rs).e().read();
                    self.bit_operation(action, rd, index, matches!(uop, Uop::BitR16(..)));
                }
                Uop::BitRRMW(action, addr, index) => {
                    let addr = self.state.reg(addr).e().read();
                    let index = self.state.reg(index).e().read();
                    let offset = ((index as i32) >> 5) << 2;
                    let addr = addr.wrapping_add(offset as u32);
                    self.bit_memory_update(action, addr, index, false)?;
                }
                Uop::BitRRMW16(action, addr, index) => {
                    let addr = self.state.reg(addr).e().read();
                    let index = self.state.reg(index).e().read();
                    let offset = ((index as i16 as i32) >> 4) << 1;
                    let addr = addr.wrapping_add(offset as u32);
                    self.bit_memory_update(action, addr, index, true)?;
                }
                Uop::BsfR(rd, rs) => {
                    let src = self.state.reg(rs).e().read();
                    self.state.flags_mut().set_dynamic(Flags::ZF, src == 0);
                    if src != 0 {
                        let result = src.trailing_zeros();
                        self.state.reg(rd).e().write(result as u32);
                    }
                }
                Uop::BsfR16(rd, rs) => {
                    let src = self.state.reg(rs).w().read();
                    self.state.flags_mut().set_dynamic(Flags::ZF, src == 0);
                    if src != 0 {
                        let result = src.trailing_zeros();
                        self.state.reg(rd).w().write(result as u16);
                    }
                }
                Uop::BsrR(rd, rs) => {
                    let src = self.state.reg(rs).e().read();
                    self.state.flags_mut().set_dynamic(Flags::ZF, src == 0);
                    if src != 0 {
                        let result = 31 - src.leading_zeros();
                        self.state.reg(rd).e().write(result as u32);
                    }
                }
                Uop::BsrR16(rd, rs) => {
                    let src = self.state.reg(rs).w().read();
                    self.state.flags_mut().set_dynamic(Flags::ZF, src == 0);
                    if src != 0 {
                        let result = 15 - src.leading_zeros();
                        self.state.reg(rd).w().write(result as u16);
                    }
                }
                Uop::BswapR(rd) => {
                    let value = self.state.reg(rd).e().read().swap_bytes();
                    self.state.reg(rd).e().write(value);
                }
                Uop::Call(func_index, return_addr) => {
                    let (eip, cached) = self.tracer.functions()[func_index.0 as usize];
                    self.push(return_addr.0).ok_or(Exception::StackFault)?;
                    if self.pending_write() {
                        self.flat_branch(eip);
                    } else {
                        let upc = if cached != AddrIndex::BAD_ADDRESS {
                            cached
                        } else {
                            let upc = self.resolve_flat_target(eip)?;
                            self.tracer.set_function_target(func_index, upc);
                            upc
                        };
                        self.tracer.set_current_upc(upc);
                    }
                    continue;
                }
                Uop::CallR(rd, return_addr) => {
                    let target = self.state.reg(rd).e().read();
                    self.push(return_addr.0).ok_or(Exception::StackFault)?;
                    self.flat_branch(Offset32(target));
                    continue;
                }

                Uop::Cwd => {
                    let sign = (self.state.ax().read() as i16) >> 15;
                    self.state.dx().write(sign as u16);
                }
                Uop::Cdq => {
                    let eax = self.state.eax().read() as i32;
                    self.state.edx().write(if eax < 0 { u32::MAX } else { 0 });
                }
                Uop::Cmov(cc, rd, rs) => {
                    if self.state.eval_cc(cc) {
                        let value = self.state.reg(rs).e().read();
                        self.state.reg(rd).e().write(value);
                    }
                }
                Uop::Cmov16(cc, rd, rs) => {
                    if self.state.eval_cc(cc) {
                        let value = self.state.reg(rs).w().read();
                        self.state.reg(rd).w().write(value);
                    }
                }
                Uop::CmpI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    self.alu().sub32(dst, id);
                }
                Uop::CmpI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    self.alu().sub16(dst, iw);
                }

                Uop::CmpI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    self.alu().sub8(dst, ib);
                }
                Uop::CmpR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    self.alu().sub32(dst, src);
                }
                Uop::CmpR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    self.alu().sub16(dst, src);
                }
                Uop::CmpR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    self.alu().sub8(dst, src);
                }
                Uop::CmpxchgR(rd, rs) => {
                    let src = self.state.reg(rs).e().read();
                    let acc = self.state.eax().read();
                    let dst = self.state.reg(rd).e().read();
                    self.alu().sub32(acc, dst);
                    if acc == dst {
                        self.state.reg(rd).e().write(src);
                    } else {
                        self.state.eax().write(dst);
                    }
                }
                Uop::CmpxchgRMW(addr, rs) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = self.state.reg(rs).e().read();
                    let acc = self.state.eax().read();
                    let dst = self.read_memory32(addr)?;
                    // CMPXCHG writes the destination even when comparison fails.
                    // Check/write memory before changing guest registers or flags.
                    self.write_memory32(addr, if acc == dst { src } else { dst })?;
                    self.alu().sub32(acc, dst);
                    if acc != dst {
                        self.state.eax().write(dst);
                    }
                }
                Uop::CmpxchgR16(rd, rs) => {
                    let src = self.state.reg(rs).w().read();
                    let acc = self.state.ax().read();
                    let dst = self.state.reg(rd).w().read();
                    self.alu().sub16(acc, dst);
                    if acc == dst {
                        self.state.reg(rd).w().write(src);
                    } else {
                        self.state.ax().write(dst);
                    }
                }
                Uop::CmpxchgRMW16(addr, rs) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = self.state.reg(rs).w().read();
                    let acc = self.state.ax().read();
                    let dst = self.read_memory16(addr)?;
                    // CMPXCHG writes the destination even when comparison fails.
                    // Check/write memory before changing guest registers or flags.
                    self.write_memory16(addr, if acc == dst { src } else { dst })?;
                    self.alu().sub16(acc, dst);
                    if acc != dst {
                        self.state.ax().write(dst);
                    }
                }
                Uop::CmpxchgR8(rd, rs) => {
                    let src = self.state.reg8(rs).read();
                    let acc = self.state.al().read();
                    let dst = self.state.reg8(rd).read();
                    self.alu().sub8(acc, dst);
                    if acc == dst {
                        self.state.reg8(rd).write(src);
                    } else {
                        self.state.al().write(dst);
                    }
                }
                Uop::CmpxchgRMW8(addr, rs) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = self.state.reg8(rs).read();
                    let acc = self.state.al().read();
                    let dst = self.read_memory8(addr)?;
                    // CMPXCHG writes the destination even when comparison fails.
                    // Check/write memory before changing guest registers or flags.
                    self.write_memory8(addr, if acc == dst { src } else { dst })?;
                    self.alu().sub8(acc, dst);
                    if acc != dst {
                        self.state.al().write(dst);
                    }
                }
                Uop::DecR(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().dec32(dst);
                    self.state.reg(rd).e().write(result);
                }
                Uop::DecR16(rd) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().dec16(dst);
                    self.state.reg(rd).w().write(result);
                }
                Uop::DecR8(rd) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().dec8(dst);
                    self.state.reg8(rd).write(result);
                }

                Uop::DivR(rs) => {
                    let src = self.state.reg(rs).e().read();
                    self.divide(src, 32, false)?;
                }
                Uop::DivR16(rs) => {
                    let src = self.state.reg(rs).w().read() as u32;
                    self.divide(src, 16, false)?;
                }
                Uop::DivR8(rs) => {
                    let src = self.state.reg8(rs).read() as u32;
                    self.divide(src, 8, false)?;
                }

                // Invalid for here
                Uop::FetchNext(_) => return Err(Exception::Unimplemented2(uop)),

                Uop::IDivR(rs) => {
                    let src = self.state.reg(rs).e().read();
                    self.divide(src, 32, true)?;
                }
                Uop::IDivR16(rs) => {
                    let src = self.state.reg(rs).w().read() as u32;
                    self.divide(src, 16, true)?;
                }
                Uop::IDivR8(rs) => {
                    let src = self.state.reg8(rs).read() as u32;
                    self.divide(src, 8, true)?;
                }
                Uop::IMulFullR(rs) => {
                    let src = self.state.reg(rs).e().read() as i32;
                    let acc = self.state.eax().read() as i32;
                    let result = (acc as i64) * (src as i64);
                    self.alu().imul32(acc, src);
                    self.state.eax().write(result as u32);
                    self.state.edx().write((result >> 32) as u32);
                }
                Uop::IMulFullR16(rs) => {
                    let src = self.state.reg(rs).w().read() as i16;
                    let acc = self.state.ax().read() as i16;
                    let result = (acc as i64) * (src as i64);
                    self.alu().imul16(acc, src);
                    self.state.ax().write(result as u16);
                    self.state.reg(RtReg::EDX).w().write((result >> 16) as u16);
                }
                Uop::IMulFullR8(rs) => {
                    let src = self.state.reg8(rs).read() as i8;
                    let acc = self.state.al().read() as i8;
                    let result = (acc as i64) * (src as i64);
                    self.alu().imul8(acc, src);
                    self.state.ax().write(result as u16);
                }

                Uop::IMulR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read() as i32;
                    let src = self.state.reg(rs).e().read() as i32;
                    let result = self.alu().imul32(dst, src);
                    self.state.reg(rd).e().write(result as u32);
                }
                Uop::IMulR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read() as i16;
                    let src = self.state.reg(rs).w().read() as i16;
                    let result = self.alu().imul16(dst, src);
                    self.state.reg(rd).w().write(result as u16);
                }
                Uop::IMulRI(rd, rs, id) => {
                    let src = self.state.reg(rs).e().read() as i32;
                    let result = self.alu().imul32(src, id);
                    self.state.reg(rd).e().write(result as u32);
                }
                Uop::IMulRI16(rd, rs, imm) => {
                    let src = self.state.reg(rs).w().read() as i16;
                    let result = self.alu().imul16(src, imm);
                    self.state.reg(rd).w().write(result as u16);
                }
                Uop::IncR(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().inc32(dst);
                    self.state.reg(rd).e().write(result);
                }
                Uop::IncR16(rd) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().inc16(dst);
                    self.state.reg(rd).w().write(result);
                }
                Uop::IncR8(rd) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().inc8(dst);
                    self.state.reg8(rd).write(result);
                }

                Uop::Jcc(cc, target) => {
                    if self.state.eval_cc(cc) {
                        self.tracer.set_current_upc(target);
                        continue;
                    }
                }
                Uop::JccU(cc, target) => {
                    if self.state.eval_cc(cc) {
                        let addr_index = self.resolve_flat_target(target)?;
                        self.tracer.replace(TraceDecoder::jcc_opt(cc, addr_index));
                        self.tracer.set_current_upc(addr_index);
                        continue;
                    }
                }
                Uop::Jnz(target) => {
                    if !self.state.flags().zf() {
                        self.tracer.set_current_upc(target);
                        continue;
                    }
                }

                Uop::JumpU(eip) => {
                    let target = self.resolve_flat_target(eip)?;
                    self.tracer.replace(Uop::Jump(target));
                    self.tracer.set_current_upc(target);
                    continue;
                }
                Uop::CheckWrite(next) => {
                    if self.pending_write() {
                        self.flat_branch(next);
                        continue;
                    }
                }
                Uop::Jump(target) => {
                    self.tracer.set_current_upc(target);
                    continue;
                }
                Uop::JumpR(rs) => {
                    let target = self.state.reg(rs).e().read();
                    self.flat_branch(Offset32(target));
                    continue;
                }
                Uop::Jz(target) => {
                    if self.state.flags().zf() {
                        self.tracer.set_current_upc(target);
                        continue;
                    }
                }

                Uop::LeaBD(rd, rb, disp) => {
                    let base = self.state.reg(rb).e().read();
                    let addr = base.wrapping_add(disp.0);
                    self.state.reg(rd).e().write(addr);
                }
                Uop::LeaSIB(rd, sib, disp) => {
                    let addr = self.resolve_sib(sib).wrapping_add(disp.0);
                    self.state.reg(rd).e().write(addr);
                }
                Uop::LoadBD32(rd, rb, disp) => {
                    let base = self.state.reg(rb).e().read();
                    let addr = base.wrapping_add(disp.0);
                    let value = self.read_memory32(addr)?;
                    self.state.reg(rd).e().write(value);
                }
                Uop::LoadConst(rd, id) => {
                    self.state.reg(rd).e().write(id);
                }
                Uop::LoadConst16(rd, iw) => {
                    self.state.reg(rd).w().write(iw);
                }

                Uop::LoadConst8(rd, ib) => {
                    self.state.reg8(rd).write(ib);
                }
                Uop::LoadExtend16(extension, rd, rb) => {
                    let addr = self.state.reg(rb).e().read();
                    let value = self.read_memory16(addr)?;
                    let value = match extension {
                        uop::LoadExtension::Zero => value as u32,
                        uop::LoadExtension::Sign => value as i16 as i32 as u32,
                    };
                    self.state.reg(rd).e().write(value);
                }
                Uop::LoadExtend8(extension, rd, rb) => {
                    let addr = self.state.reg(rb).e().read();
                    let value = self.read_memory8(addr)?;
                    let value = match extension {
                        uop::LoadExtension::Zero => value as u32,
                        uop::LoadExtension::Sign => value as i8 as i32 as u32,
                    };
                    self.state.reg(rd).e().write(value);
                }
                Uop::LoadR16(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let value = self.read_memory16(base)?;
                    self.state.reg(rd).w().write(value);
                }
                Uop::AddLoad(rd, rb) => {
                    let addr = self.state.reg(rb).e().read();
                    let src = self.read_memory32(addr)?;
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().add32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::SubLoad(rd, rb) => {
                    let addr = self.state.reg(rb).e().read();
                    let src = self.read_memory32(addr)?;
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().sub32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::AndLoad(rd, rb) => {
                    let addr = self.state.reg(rb).e().read();
                    let src = self.read_memory32(addr)?;
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().and32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::OrLoad(rd, rb) => {
                    let addr = self.state.reg(rb).e().read();
                    let src = self.read_memory32(addr)?;
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().or32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::XorLoad(rd, rb) => {
                    let addr = self.state.reg(rb).e().read();
                    let src = self.read_memory32(addr)?;
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().xor32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::CmpRegMem(rd, rb) => {
                    let addr = self.state.reg(rb).e().read();
                    let mem = self.read_memory32(addr)?;
                    let reg = self.state.reg(rd).e().read();
                    self.alu().sub32(reg, mem);
                }
                Uop::CmpMemReg(rb, rs) => {
                    let addr = self.state.reg(rb).e().read();
                    let mem = self.read_memory32(addr)?;
                    let reg = self.state.reg(rs).e().read();
                    self.alu().sub32(mem, reg);
                }
                Uop::TestMemReg(rb, rs) => {
                    let addr = self.state.reg(rb).e().read();
                    let mem = self.read_memory32(addr)?;
                    let reg = self.state.reg(rs).e().read();
                    self.alu().and32(mem, reg);
                }
                Uop::LoadR32(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let value = self.read_memory32(base)?;
                    self.state.reg(rd).e().write(value);
                }

                Uop::LoadR8(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let value = self.read_memory8(base)?;
                    self.state.reg8(rd).write(value);
                }
                Uop::LoadSIB32(rd, sib, disp) => {
                    let addr = self.resolve_sib(sib).wrapping_add(disp.0);
                    let value = self.read_memory32(addr)?;
                    self.state.reg(rd).e().write(value);
                }
                Uop::Loop(kind, addr32, target) => {
                    let count = if addr32 {
                        self.state.ecx().read()
                    } else {
                        u32::from(self.state.cx().read())
                    };
                    let taken = if matches!(kind, LoopKind::CountZero) {
                        count == 0
                    } else {
                        let count = count.wrapping_sub(1) & if addr32 { u32::MAX } else { 0xffff };
                        if addr32 {
                            self.state.ecx().write(count);
                        } else {
                            self.state.cx().write(count as u16);
                        }
                        count != 0
                            && match kind {
                                LoopKind::WhileZero => self.state.flags().zf(),
                                LoopKind::WhileNotZero => !self.state.flags().zf(),
                                _ => true,
                            }
                    };
                    if taken {
                        self.flat_branch(target);
                        continue;
                    }
                }
                Uop::MovSx16(rd, rs) => {
                    let value = self.state.reg(rs).w().read() as i16 as i32 as u32;
                    self.state.reg(rd).e().write(value);
                }
                Uop::MovSx8(rd, rs) => {
                    let value = self.state.reg8(rs).read() as i8 as i32 as u32;
                    self.state.reg(rd).e().write(value);
                }
                Uop::MovZx16(rd, rs) => {
                    let value = self.state.reg(rs).w().read();
                    self.state.reg(rd).e().write(value as u32);
                }
                Uop::MovZx8(rd, rs) => {
                    let value = self.state.reg8(rs).read();
                    self.state.reg(rd).e().write(value as u32);
                }

                Uop::Move(rd, rs) => {
                    let value = self.state.reg(rs).e().read();
                    self.state.reg(rd).e().write(value);
                }
                Uop::Move16(rd, rs) => {
                    let value = self.state.reg(rs).w().read();
                    self.state.reg(rd).w().write(value);
                }
                Uop::Move8(rd, rs) => {
                    let value = self.state.reg8(rs).read();
                    self.state.reg8(rd).write(value);
                }
                Uop::MulR(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let eax = self.state.eax().read();
                    let result = self.alu().mul32(eax, dst);
                    self.state.eax().write(result as u32);
                    self.state.edx().write((result >> 32) as u32);
                }
                Uop::MulR16(rs) => {
                    let src = self.state.reg(rs).w().read();
                    let acc = self.state.ax().read();
                    let result = self.alu().mul16(acc, src);
                    self.state.ax().write(result as u16);
                    self.state.reg(RtReg::EDX).w().write((result >> 16) as u16);
                }
                Uop::MulR8(rs) => {
                    let src = self.state.reg8(rs).read();
                    let acc = self.state.al().read();
                    let result = self.alu().mul8(acc, src);
                    self.state.ax().write(result);
                }
                Uop::NegR(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().sub32(0, dst);
                    self.state.reg(rd).e().write(result);
                }
                Uop::NegR16(rd) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().sub16(0, dst);
                    self.state.reg(rd).w().write(result);
                }
                Uop::NegR8(rd) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().sub8(0, dst);
                    self.state.reg8(rd).write(result);
                }

                Uop::Nop => {}
                Uop::NotR(rd) => {
                    let value = self.state.reg(rd).e().read();
                    self.state.reg(rd).e().write(!value);
                }

                Uop::OrI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().or32(dst, id);
                    self.state.reg(rd).e().write(result);
                }
                Uop::OrI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().or16(dst, iw);
                    self.state.reg(rd).w().write(result);
                }

                Uop::OrI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().or8(dst, ib);
                    self.state.reg8(rd).write(result);
                }

                Uop::OrR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().or32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::OrR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().or16(dst, src);
                    self.state.reg(rd).w().write(result);
                }
                Uop::OrR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().or8(dst, src);
                    self.state.reg8(rd).write(result);
                }
                Uop::OrRMW(rd, rs) => {
                    let addr = self.state.reg(rd).e().read();
                    let dst = self.read_memory32(addr)?;
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().or32(dst, src);
                    self.write_memory32(addr, result)?;
                }

                Uop::PopMemR(base, disp) => {
                    self.pop_memory_destination(Some(base), None, disp.0)?;
                }
                Uop::PopMemSIB(sib, disp) => {
                    self.pop_memory_destination(None, Some(sib), disp.0)?;
                }

                Uop::PopR(rd) => {
                    let value = self.pop().ok_or(Exception::StackFault)?;
                    self.state.reg(rd).e().write(value);
                }
                Uop::Popcnt(rd, rs) => {
                    let value = self.state.reg(rs).e().read();
                    self.state.reg(rd).e().write(value.count_ones());
                    self.popcount_flags(value == 0);
                }
                Uop::Popcnt16(rd, rs) => {
                    let value = self.state.reg(rs).w().read();
                    self.state.reg(rd).w().write(value.count_ones() as u16);
                    self.popcount_flags(value == 0);
                }

                Uop::PushI(id) => {
                    self.push(id).ok_or(Exception::StackFault)?;
                }
                Uop::PushR(rd) => {
                    let value = self.state.reg(rd).e().read();
                    self.push(value).ok_or(Exception::StackFault)?;
                }
                Uop::Ret(iw) => {
                    let return_addr = self.pop().ok_or(Exception::StackFault)?;
                    self.state().esp().modify(|v| v.wrapping_add(iw as u32));
                    self.flat_branch(Offset32(return_addr));
                    continue;
                }
                Uop::RolI(rd, count) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().rol32(dst, count);
                    self.state.reg(rd).e().write(result);
                }
                Uop::RolI16(rd, count) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().rol16(dst, count);
                    self.state.reg(rd).w().write(result);
                }
                Uop::RolI8(rd, count) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().rol8(dst, count);
                    self.state.reg8(rd).write(result);
                }
                Uop::RolIRMW(addr, count) => {
                    let addr = self.state.reg(addr).e().read();
                    let dst = self.read_memory32(addr)?;
                    let result = self.alu().rol32(dst, count);
                    self.write_memory32(addr, result)?;
                }

                Uop::RolRCl(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let count = self.state.cl().read();
                    let result = self.alu().rol32(dst, count);
                    self.state.reg(rd).e().write(result);
                }
                Uop::RolRCl16(rd) => {
                    let dst = self.state.reg(rd).w().read();
                    let count = self.state.cl().read();
                    let result = self.alu().rol16(dst, count);
                    self.state.reg(rd).w().write(result);
                }
                Uop::RolRCl8(rd) => {
                    let dst = self.state.reg8(rd).read();
                    let count = self.state.cl().read();
                    let result = self.alu().rol8(dst, count);
                    self.state.reg8(rd).write(result);
                }

                Uop::RorI(rd, count) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().ror32(dst, count);
                    self.state.reg(rd).e().write(result);
                }
                Uop::RorI16(rd, count) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().ror16(dst, count);
                    self.state.reg(rd).w().write(result);
                }
                Uop::RorI8(rd, count) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().ror8(dst, count);
                    self.state.reg8(rd).write(result);
                }

                Uop::RorRCl(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let count = self.state.cl().read();
                    let result = self.alu().ror32(dst, count);
                    self.state.reg(rd).e().write(result);
                }
                Uop::RorRCl16(rd) => {
                    let dst = self.state.reg(rd).w().read();
                    let count = self.state.cl().read();
                    let result = self.alu().ror16(dst, count);
                    self.state.reg(rd).w().write(result);
                }
                Uop::RorRCl8(rd) => {
                    let dst = self.state.reg8(rd).read();
                    let count = self.state.cl().read();
                    let result = self.alu().ror8(dst, count);
                    self.state.reg8(rd).write(result);
                }

                Uop::SarI(rd, ib) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().sar32(dst, ib);
                    self.state.reg(rd).e().write(result);
                }
                Uop::SarI16(rd, ib) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().sar16(dst, ib);
                    self.state.reg(rd).w().write(result);
                }

                Uop::SarI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().sar8(dst, ib);
                    self.state.reg8(rd).write(result);
                }

                Uop::SarRCl(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().sar32(dst, cl);
                    self.state.reg(rd).e().write(result);
                }
                Uop::SarRCl16(rd) => {
                    let dst = self.state.reg(rd).w().read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().sar16(dst, cl);
                    self.state.reg(rd).w().write(result);
                }
                Uop::SarRCl8(rd) => {
                    let dst = self.state.reg8(rd).read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().sar8(dst, cl);
                    self.state.reg8(rd).write(result);
                }

                Uop::SbbI(rd, imm) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().sbb32(dst, imm);
                    self.state.reg(rd).e().write(result);
                }
                Uop::SbbI16(rd, imm) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().sbb16(dst, imm);
                    self.state.reg(rd).w().write(result);
                }
                Uop::SbbI8(rd, imm) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().sbb8(dst, imm);
                    self.state.reg8(rd).write(result);
                }

                Uop::SbbR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().sbb32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::SbbR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().sbb16(dst, src);
                    self.state.reg(rd).w().write(result);
                }
                Uop::SbbR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().sbb8(dst, src);
                    self.state.reg8(rd).write(result);
                }

                Uop::SetCC(cc, rd) => {
                    let value = self.state.eval_cc(cc) as u8;
                    self.state.reg8(rd).write(value);
                }
                Uop::ShiftIRMW(op, addr, count) => {
                    let addr = self.state.reg(addr).e().read();
                    self.shift_memory_update(op, addr, count)?;
                }

                Uop::ShiftRMWCl(op, addr) => {
                    let addr = self.state.reg(addr).e().read();
                    let count = self.state.cl().read();
                    self.shift_memory_update(op, addr, count)?;
                }

                Uop::ShlI(rd, ib) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().shl32(dst, ib);
                    self.state.reg(rd).e().write(result);
                }
                Uop::ShlI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().shl16(dst, iw);
                    self.state.reg(rd).w().write(result);
                }

                Uop::ShlI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().shl8(dst, ib);
                    self.state.reg8(rd).write(result);
                }

                Uop::ShlRCl(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().shl32(dst, cl);
                    self.state.reg(rd).e().write(result);
                }
                Uop::ShlRCl16(rd) => {
                    let dst = self.state.reg(rd).w().read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().shl16(dst, cl);
                    self.state.reg(rd).w().write(result);
                }
                Uop::ShlRCl8(rd) => {
                    let dst = self.state.reg8(rd).read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().shl8(dst, cl);
                    self.state.reg8(rd).write(result);
                }

                Uop::ShldI(rd, rs, count) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().shld32(dst, src, count);
                    self.state.reg(rd).e().write(result);
                }

                Uop::ShldI16(rd, rs, count) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().shld16(dst, src, count);
                    self.state.reg(rd).w().write(result);
                }

                Uop::ShldRCl(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let count = self.state.cl().read();
                    let result = self.alu().shld32(dst, src, count);
                    self.state.reg(rd).e().write(result);
                }
                Uop::ShldRCl16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let count = self.state.cl().read();
                    let result = self.alu().shld16(dst, src, count);
                    self.state.reg(rd).w().write(result);
                }

                Uop::ShrI(rd, ib) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().shr32(dst, ib);
                    self.state.reg(rd).e().write(result);
                }
                Uop::ShrI16(rd, ib) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().shr16(dst, ib);
                    self.state.reg(rd).w().write(result);
                }

                Uop::ShrI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().shr8(dst, ib);
                    self.state.reg8(rd).write(result);
                }

                Uop::ShrRCl(rd) => {
                    let dst = self.state.reg(rd).e().read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().shr32(dst, cl);
                    self.state.reg(rd).e().write(result);
                }
                Uop::ShrRCl16(rd) => {
                    let dst = self.state.reg(rd).w().read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().shr16(dst, cl);
                    self.state.reg(rd).w().write(result);
                }
                Uop::ShrRCl8(rd) => {
                    let dst = self.state.reg8(rd).read();
                    let cl = self.state.cl().read() as u8;
                    let result = self.alu().shr8(dst, cl);
                    self.state.reg8(rd).write(result);
                }

                Uop::ShrdI(rd, rs, count) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().shrd32(dst, src, count);
                    self.state.reg(rd).e().write(result);
                }
                Uop::ShrdI16(rd, rs, count) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().shrd16(dst, src, count);
                    self.state.reg(rd).w().write(result);
                }

                Uop::ShrdRCl(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let count = self.state.cl().read();
                    let result = self.alu().shrd32(dst, src, count);
                    self.state.reg(rd).e().write(result);
                }
                Uop::ShrdRCl16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let count = self.state.cl().read();
                    let result = self.alu().shrd16(dst, src, count);
                    self.state.reg(rd).w().write(result);
                }

                Uop::StoreBD32(rd, rb, disp) => {
                    let base = self.state.reg(rb).e().read();
                    let addr = base.wrapping_add(disp.0);
                    let value = self.state.reg(rd).e().read();
                    self.write_memory32(addr, value)?;
                }
                Uop::StoreR16(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let value = self.state.reg(rd).w().read();
                    self.write_memory16(base, value)?;
                }
                Uop::StoreR32(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let value = self.state.reg(rd).e().read();
                    self.write_memory32(base, value)?;
                }

                Uop::StoreR8(rd, rb) => {
                    let base = self.state.reg(rb).e().read();
                    let value = self.state.reg8(rd).read();
                    self.write_memory8(base, value)?;
                }
                Uop::StoreSIB32(rd, sib, disp) => {
                    let addr = self.resolve_sib(sib).wrapping_add(disp.0);
                    let value = self.state.reg(rd).e().read();
                    self.write_memory32(addr, value)?;
                }
                Uop::String(op) => {
                    self.execute_string(op)?;
                }
                Uop::SubI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().sub32(dst, id);
                    self.state.reg(rd).e().write(result);
                }
                Uop::SubI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().sub16(dst, iw);
                    self.state.reg(rd).w().write(result);
                }

                Uop::SubI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().sub8(dst, ib);
                    self.state.reg8(rd).write(result);
                }

                Uop::SubR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().sub32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::SubR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().sub16(dst, src);
                    self.state.reg(rd).w().write(result);
                }
                Uop::SubR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().sub8(dst, src);
                    self.state.reg8(rd).write(result);
                }
                Uop::SubRMW(rd, rs) => {
                    let addr = self.state.reg(rd).e().read();
                    let dst = self.read_memory32(addr)?;
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().sub32(dst, src);
                    self.write_memory32(addr, result)?;
                }

                Uop::SubRMW8(rd, rs) => {
                    let addr = self.state.reg(rd).e().read();
                    let dst = self.read_memory8(addr)?;
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().sub8(dst, src);
                    self.write_memory8(addr, result)?;
                }

                Uop::SysCall => {
                    return Err(Exception::SysCall);
                }
                Uop::SysEnter => {
                    return Err(Exception::SysEnter);
                }
                Uop::Swi(ib) => {
                    return Err(Exception::Swi(ib));
                }

                Uop::TestI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    self.alu().and32(dst, id);
                }
                Uop::TestI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    self.alu().and16(dst, iw);
                }

                Uop::TestI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    self.alu().and8(dst, ib);
                }
                Uop::TestR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    self.alu().and32(dst, src);
                }
                Uop::TestR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    self.alu().and16(dst, src);
                }
                Uop::TestR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    self.alu().and8(dst, src);
                }
                Uop::UnaryRMW(op, addr) => {
                    let addr = self.state.reg(addr).e().read();
                    self.unary_memory_update(op, addr)?;
                }

                // Data memory has identical read/write bounds and no separate
                // write protection. Once this read succeeds, the same-width
                // write cannot fault. Keep the address before modifying rs:
                // it may also be a base/index register of the memory operand.
                Uop::XaddRMW(addr, rs) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = self.state.reg(rs).e().read();
                    let dst = self.read_memory32(addr)?;
                    let sum = self.alu().add32(dst, src);
                    self.write_memory32(addr, sum)?;
                    self.state.reg(rs).e().write(dst);
                }
                Uop::XaddRMW16(addr, rs) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = self.state.reg(rs).w().read();
                    let dst = self.read_memory16(addr)?;
                    let sum = self.alu().add16(dst, src);
                    self.write_memory16(addr, sum)?;
                    self.state.reg(rs).w().write(dst);
                }
                Uop::XaddRMW8(addr, rs) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = self.state.reg8(rs).read();
                    let dst = self.read_memory8(addr)?;
                    let sum = self.alu().add8(dst, src);
                    self.write_memory8(addr, sum)?;
                    self.state.reg8(rs).write(dst);
                }
                Uop::XaddR(rd, rs) => self.alu().xadd32(rd, rs),
                Uop::XaddR16(rd, rs) => self.alu().xadd16(rd, rs),
                Uop::XaddR8(rd, rs) => self.alu().xadd8(rd, rs),
                Uop::XchgR(rd, rs) => {
                    let value_rd = self.state.reg(rd).e().read();
                    let value_rs = self.state.reg(rs).e().read();
                    self.state.reg(rd).e().write(value_rs);
                    self.state.reg(rs).e().write(value_rd);
                }
                Uop::XchgR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    self.state.reg(rd).w().write(src);
                    self.state.reg(rs).w().write(dst);
                }
                Uop::XchgR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    self.state.reg8(rd).write(src);
                    self.state.reg8(rs).write(dst);
                }
                Uop::XchgRMW(addr, rs) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = self.state.reg(rs).e().read();
                    let dst = self.read_memory32(addr)?;
                    self.write_memory32(addr, src)?;
                    self.state.reg(rs).e().write(dst);
                }
                Uop::XchgRMW16(addr, rs) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = self.state.reg(rs).w().read();
                    let dst = self.read_memory16(addr)?;
                    self.write_memory16(addr, src)?;
                    self.state.reg(rs).w().write(dst);
                }
                Uop::XchgRMW8(addr, rs) => {
                    let addr = self.state.reg(addr).e().read();
                    let src = self.state.reg8(rs).read();
                    let dst = self.read_memory8(addr)?;
                    self.write_memory8(addr, src)?;
                    self.state.reg8(rs).write(dst);
                }
                Uop::XorI(rd, id) => {
                    let dst = self.state.reg(rd).e().read();
                    let result = self.alu().xor32(dst, id);
                    self.state.reg(rd).e().write(result);
                }
                Uop::XorI16(rd, iw) => {
                    let dst = self.state.reg(rd).w().read();
                    let result = self.alu().xor16(dst, iw);
                    self.state.reg(rd).w().write(result);
                }

                Uop::XorI8(rd, ib) => {
                    let dst = self.state.reg8(rd).read();
                    let result = self.alu().xor8(dst, ib);
                    self.state.reg8(rd).write(result);
                }

                Uop::XorR(rd, rs) => {
                    let dst = self.state.reg(rd).e().read();
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().xor32(dst, src);
                    self.state.reg(rd).e().write(result);
                }
                Uop::XorR16(rd, rs) => {
                    let dst = self.state.reg(rd).w().read();
                    let src = self.state.reg(rs).w().read();
                    let result = self.alu().xor16(dst, src);
                    self.state.reg(rd).w().write(result);
                }
                Uop::XorR8(rd, rs) => {
                    let dst = self.state.reg8(rd).read();
                    let src = self.state.reg8(rs).read();
                    let result = self.alu().xor8(dst, src);
                    self.state.reg8(rd).write(result);
                }
                Uop::XorRMW(rd, rs) => {
                    let addr = self.state.reg(rd).e().read();
                    let dst = self.read_memory32(addr)?;
                    let src = self.state.reg(rs).e().read();
                    let result = self.alu().xor32(dst, src);
                    self.write_memory32(addr, result)?;
                }
                // After enum sorting (2026-10-09), the alphabetized Minor match owns the
                // scalar bodies: Wasm flag/BCD/XLAT workloads favored removing execute_misc.
                // Native dispatch layout remains a tradeoff; see docs/PERFORMANCE.md.
                Uop::Minor(mop) => match mop {
                    UopMinor::Aaa => self.alu().ascii_adjust(false),
                    UopMinor::Aad(base) => self.alu().aad(base),
                    UopMinor::Aam(base) => {
                        if self.alu().aam(base).is_none() {
                            return Err(Exception::DivisionError);
                        }
                    }
                    UopMinor::Aas => self.alu().ascii_adjust(true),
                    UopMinor::Clc => {
                        self.state.flags_mut().set_dynamic(Flags::CF, false);
                    }
                    UopMinor::Cld => {
                        self.state_mut().flags_mut().set_static(Flags::DF, false);
                    }
                    UopMinor::Cmc => {
                        self.state.flags_mut().complement_cf();
                    }
                    UopMinor::Cpuid => {
                        self.state.invoke_cpuid();
                    }
                    UopMinor::Daa => self.alu().decimal_adjust(false),
                    UopMinor::Das => self.alu().decimal_adjust(true),
                    UopMinor::Enter(bytes) => self.enter_zero(bytes)?,
                    UopMinor::Leave => {
                        let frame = self.state.ebp().read();
                        let value = self
                            .read_memory32(frame)
                            .map_err(|_| Exception::StackFault)?;
                        self.state.esp().write(frame.wrapping_add(4));
                        self.state.ebp().write(value);
                    }
                    UopMinor::InvalidOpcode => return Err(Exception::InvalidOpcode),
                    UopMinor::PortIo => {
                        return Err(Exception::Unimplemented2(Uop::Minor(mop)));
                    }
                    UopMinor::Lahf => {
                        let value = (self.state.flags_mut().resolve().bits() & 0xd5) | 2;
                        self.state.ah().write(value as u8);
                    }
                    UopMinor::PopAd => {
                        self.popad().ok_or(Exception::StackFault)?;
                    }
                    UopMinor::PopFd => {
                        let esp = self.state.esp().read();
                        let value = self.read_memory32(esp).map_err(|_| Exception::StackFault)?;
                        // Deliberate FlatUME policy: no POPFD-specific privilege masks or
                        // flag adjustments. Pass the stack image directly to set_bits.
                        self.state.flags_mut().set_bits(Flags::from_bits(value));
                        self.state.esp().write(esp.wrapping_add(4));
                    }
                    UopMinor::PushAd => {
                        self.pushad().ok_or(Exception::StackFault)?;
                    }
                    UopMinor::PushFd => {
                        let value = self.state.flags_mut().resolve().bits()
                            & !(Flags::RF | Flags::VM).bits();
                        let esp = self.state.esp().read().wrapping_sub(4);
                        self.write_memory32(esp, value)
                            .map_err(|_| Exception::StackFault)?;
                        self.state.esp().write(esp);
                    }
                    UopMinor::RclI(rd, count) => {
                        let dst = self.state.reg(rd).e().read();
                        let result = self.alu().rcl32(dst, count);
                        self.state.reg(rd).e().write(result);
                    }
                    UopMinor::RclI16(rd, count) => {
                        let dst = self.state.reg(rd).w().read();
                        let result = self.alu().rcl16(dst, count);
                        self.state.reg(rd).w().write(result);
                    }
                    UopMinor::RclI8(rd, count) => {
                        let dst = self.state.reg8(rd).read();
                        let result = self.alu().rcl8(dst, count);
                        self.state.reg8(rd).write(result);
                    }

                    UopMinor::RclRCl(rd) => {
                        let dst = self.state.reg(rd).e().read();
                        let count = self.state.cl().read();
                        let result = self.alu().rcl32(dst, count);
                        self.state.reg(rd).e().write(result);
                    }
                    UopMinor::RclRCl16(rd) => {
                        let dst = self.state.reg(rd).w().read();
                        let count = self.state.cl().read();
                        let result = self.alu().rcl16(dst, count);
                        self.state.reg(rd).w().write(result);
                    }
                    UopMinor::RclRCl8(rd) => {
                        let dst = self.state.reg8(rd).read();
                        let count = self.state.cl().read();
                        let result = self.alu().rcl8(dst, count);
                        self.state.reg8(rd).write(result);
                    }

                    UopMinor::RcrI(rd, count) => {
                        let dst = self.state.reg(rd).e().read();
                        let result = self.alu().rcr32(dst, count);
                        self.state.reg(rd).e().write(result);
                    }
                    UopMinor::RcrI16(rd, count) => {
                        let dst = self.state.reg(rd).w().read();
                        let result = self.alu().rcr16(dst, count);
                        self.state.reg(rd).w().write(result);
                    }
                    UopMinor::RcrI8(rd, count) => {
                        let dst = self.state.reg8(rd).read();
                        let result = self.alu().rcr8(dst, count);
                        self.state.reg8(rd).write(result);
                    }

                    UopMinor::RcrRCl(rd) => {
                        let dst = self.state.reg(rd).e().read();
                        let count = self.state.cl().read();
                        let result = self.alu().rcr32(dst, count);
                        self.state.reg(rd).e().write(result);
                    }
                    UopMinor::RcrRCl16(rd) => {
                        let dst = self.state.reg(rd).w().read();
                        let count = self.state.cl().read();
                        let result = self.alu().rcr16(dst, count);
                        self.state.reg(rd).w().write(result);
                    }
                    UopMinor::RcrRCl8(rd) => {
                        let dst = self.state.reg8(rd).read();
                        let count = self.state.cl().read();
                        let result = self.alu().rcr8(dst, count);
                        self.state.reg8(rd).write(result);
                    }

                    UopMinor::RdTsc => {
                        return Err(Exception::RdTsc);
                    }
                    UopMinor::LoadSegment(sr, rs) => {
                        let selector = self.state.reg(rs).w().read();
                        return Err(Exception::LoadSegment(sr, selector));
                    }
                    UopMinor::ReadSegment(rd, sr, is_dword) => {
                        let selector = self.state.sr(sr).sel.0;
                        if is_dword {
                            self.state.reg(rd).e().write(selector as u32);
                        } else {
                            self.state.reg(rd).w().write(selector);
                        }
                    }
                    UopMinor::Sahf => {
                        self.state.flags_mut().resolve();
                        let value = self.state.ah().read() as u32;
                        for flag in [Flags::SF, Flags::ZF, Flags::AF, Flags::PF, Flags::CF] {
                            self.state
                                .flags_mut()
                                .set_dynamic(flag, value & flag.bits() != 0);
                        }
                    }
                    UopMinor::Stc => {
                        self.state.flags_mut().set_dynamic(Flags::CF, true);
                    }
                    UopMinor::Std => {
                        self.state_mut().flags_mut().set_static(Flags::DF, true);
                    }
                    UopMinor::String(op) => self.execute_string(op)?,
                    UopMinor::Todo(_) | UopMinor::Unsupported(_) => {
                        return Err(Exception::Unimplemented2(Uop::Minor(mop)));
                    }

                    UopMinor::Xlat => {
                        let addr = self
                            .state
                            .ebx()
                            .read()
                            .wrapping_add(self.state.al().read() as u32);
                        let value = self.read_memory8(addr)?;
                        self.state.al().write(value);
                    }
                },
            }
            self.tracer.advance_upc();
        }
    }

    fn divide(&mut self, divisor: u32, bits: u32, signed: bool) -> Result<(), Exception> {
        if divisor == 0 {
            return Err(Exception::DivisionError);
        }
        let dividend = match bits {
            8 => self.state.ax().read() as u64,
            16 => {
                ((self.state.reg(RtReg::EDX).w().read() as u64) << 16)
                    | self.state.ax().read() as u64
            }
            32 => ((self.state.edx().read() as u64) << 32) | self.state.eax().read() as u64,
            _ => unreachable!(),
        };
        let (quotient, remainder) = crate::alu::division::divide(dividend, divisor, bits, signed)
            .ok_or(Exception::DivisionError)?;
        match bits {
            8 => {
                self.state.al().write(quotient as u8);
                self.state.ah().write(remainder as u8);
            }
            16 => {
                self.state.ax().write(quotient as u16);
                self.state.reg(RtReg::EDX).w().write(remainder as u16);
            }
            _ => {
                self.state.eax().write(quotient);
                self.state.edx().write(remainder);
            }
        }
        Ok(())
    }
}
