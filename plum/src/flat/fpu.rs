//! x87 memory validation, architectural instruction recording and engine dispatch.
use core::ops::Range;

use super::uop::Uop;
use super::{Exception, FlatUME};
use crate::fpu::codec::{self, width};
use crate::fpu::ops::arithmetic;
use crate::fpu::uop::{FpuMinorUop, FpuOp, POP};
use crate::fpu::{FpuState, FpuValue, register_ops};
use crate::gpr::PartialRegister;
use crate::prelude::SrIndex;

impl FlatUME {
    fn fpu_range(
        &self,
        op: FpuMinorUop,
        len: usize,
        _write: bool,
    ) -> Result<Range<usize>, Exception> {
        let addr = self.state.reg(op.address()).e().read();
        self.memory_range(addr, len)
    }

    fn load_fpu(&self, op: FpuMinorUop, format: u8) -> Result<(FpuValue, u16), Exception> {
        let range = self.fpu_range(op, width(format), false)?;
        let bytes = &self.memory()[range];
        codec::decode(bytes, format).map_err(|_| Exception::Unimplemented2(Uop::Fpu(op)))
    }

    pub(super) fn execute_fpu(&mut self, op: FpuMinorUop) -> Result<(), Exception> {
        use FpuOp::*;
        #[cfg(feature = "sse")]
        if op.op == Simd {
            return self.execute_simd(op);
        }
        // Code is writable, so FOP is captured before a store can overwrite
        // its own instruction bytes and is never re-read later. The cached Uop
        // always matches the current bytes, which decoded as an ESC opcode.
        let opcode = op.records_instruction().then(|| {
            let offset = op.source_eip().wrapping_sub(self.base) as usize;
            self.memory()
                .get(offset..)
                .and_then(codec::opcode)
                .unwrap_or(0)
        });

        match op.op {
            LoadControl => {
                let range = self.fpu_range(op, 2, false)?;
                let bytes = &self.memory()[range];
                let cw = u16::from_le_bytes(bytes.try_into().unwrap());
                if cw & 0x3f != 0x3f {
                    return Err(Exception::Unimplemented2(Uop::Fpu(op)));
                }
                self.state.fpu.control = cw;
            }
            LoadEnvironment | Restore => {
                let wide = op.arg != 0;
                let full = op.op == Restore;
                let len = (if wide { 28 } else { 14 }) + if full { 80 } else { 0 };
                let range = self.fpu_range(op, len, false)?;
                let bytes = &self.memory()[range];
                let mut restored = self.state.fpu.clone();
                if !restored.restore_environment(bytes, wide, full) {
                    return Err(Exception::Unimplemented2(Uop::Fpu(op)));
                }
                self.state.fpu = restored;
            }
            StoreEnvironment | Save => {
                let wide = op.arg != 0;
                let full = op.op == Save;
                let len = (if wide { 28 } else { 14 }) + if full { 80 } else { 0 };
                let range = self.fpu_range(op, len, true)?;
                let (bytes, len) =
                    self.state
                        .fpu
                        .environment(wide, full, self.state.fpu.last_opcode);
                self.write_range(range, &bytes[..len]);
                if full {
                    self.state.fpu = FpuState::default();
                } else {
                    self.state.fpu.control |= 0x3f;
                }
            }
            StoreControl | StoreStatus => {
                let range = self.fpu_range(op, 2, true)?;
                let value = if op.op == StoreControl {
                    self.state.fpu.control_word()
                } else {
                    self.state.fpu.status_word()
                };
                self.write_range(range, &value.to_le_bytes());
            }
            Load => {
                let (value, flags) = self.load_fpu(op, op.arg)?;
                self.state.fpu.status |= flags;
                self.state.fpu.push(value);
            }
            Store => {
                let format = op.arg & 7;
                let range = self.fpu_range(op, width(format), true)?;
                let fpu = &mut self.state.fpu;
                let value = fpu.read(0).raw();
                let (bytes, flags) = codec::encode(value, op.arg, fpu.rc());
                fpu.status = (fpu.status & !0x200) | flags;
                self.write_range(range, &bytes[..width(format)]);
                if op.arg & POP != 0 {
                    self.state.fpu.pop();
                }
            }
            ArithmeticMemory => {
                if !self.state.fpu.supports_arithmetic() {
                    return Err(Exception::Unimplemented2(Uop::Fpu(op)));
                }
                let (source, flags) = self.load_fpu(op, op.arg >> 3)?;
                let lhs = self.state.fpu.read(0);
                let (value, arithmetic_flags) =
                    arithmetic(lhs, source, op.arg & 7, self.state.fpu.control);
                self.state.fpu.status = (self.state.fpu.status & !0x200) | flags | arithmetic_flags;
                self.state.fpu.set(0, value);
            }
            CompareMemory => {
                let (value, flags) = self.load_fpu(op, op.arg & 7)?;
                self.state.fpu.status |= flags;
                register_ops::compare(&mut self.state, value, false, false);
                if op.arg & POP != 0 {
                    self.state.fpu.pop();
                }
            }
            _ => register_ops::execute(&mut self.state, op)
                .map_err(|_| Exception::Unimplemented2(Uop::Fpu(op)))?,
        }

        // Only successful non-control instructions advance the architectural
        // pointers. Saves and restores must observe the preceding instruction.
        if let Some(opcode) = opcode {
            let selector = self.state.sr(SrIndex::CS).sel.0;
            let fpu = &mut self.state.fpu;
            fpu.instruction_pointer = op.source_eip();
            fpu.instruction_selector = selector;
            fpu.last_opcode = opcode;
            if op.accesses_data() {
                let pointer = self.state.reg(op.address()).e().read();
                let selector = self.state.sr(op.segment()).sel.0;
                self.state.fpu.data_pointer = pointer;
                self.state.fpu.data_selector = selector;
            }
        }
        Ok(())
    }

    /// One MMX/SSE/SSE2 form ([`crate::simd::ume`]).
    // Out of line and cold: inlined into `execute_fpu`, it made FlatUME's
    // memory-heavy benchmark cases 5-17% slower at every function alignment
    // (docs/PERFORMANCE.md).
    #[cfg(feature = "sse")]
    #[cold]
    #[inline(never)]
    fn execute_simd(&mut self, op: FpuMinorUop) -> Result<(), Exception> {
        crate::simd::ume::execute(self, op)
    }
}

#[cfg(feature = "sse")]
impl crate::simd::ume::UmeMemory for FlatUME {
    fn state(&self) -> &crate::cpu::ProcessorState {
        &self.state
    }
    fn state_mut(&mut self) -> &mut crate::cpu::ProcessorState {
        &mut self.state
    }
    fn unimplemented(op: FpuMinorUop) -> Exception {
        Exception::Unimplemented2(Uop::Fpu(op))
    }
    /// Flat: one address space, so the segment and `write` do not matter.
    fn range(
        &self,
        _segment: SrIndex,
        addr: u32,
        len: usize,
        _write: bool,
    ) -> Result<Range<usize>, Exception> {
        self.memory_range(addr, len)
    }
    fn read(&self, _segment: SrIndex, range: Range<usize>) -> &[u8] {
        &self.memory()[range]
    }
    fn write(&mut self, range: Range<usize>, bytes: &[u8]) {
        self.write_range(range, bytes);
    }
    fn fpu_opcode(&self) -> u16 {
        self.state.fpu.last_opcode
    }
}
