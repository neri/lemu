//! x87 memory validation, architectural instruction recording and engine dispatch.
use core::ops::Range;

use super::uop::Uop;
use super::{Exception, HarvardUME};
use crate::fpu::codec::{self, width};
use crate::fpu::ops::arithmetic;
use crate::fpu::uop::{FpuMinorUop, FpuOp, POP};
use crate::fpu::{FpuState, FpuValue, register_ops};
use crate::gpr::PartialRegister;
use crate::prelude::{Offset32, SrIndex};

impl HarvardUME {
    fn fpu_range(
        &self,
        op: FpuMinorUop,
        len: usize,
        write: bool,
    ) -> Result<Range<usize>, Exception> {
        self.fpu_range_at(
            op.segment(),
            self.state.reg(op.address()).e().read(),
            len,
            write,
        )
    }

    /// The host range of `len` bytes at `segment:addr`; CS is read-only.
    #[inline]
    fn fpu_range_at(
        &self,
        segment: SrIndex,
        addr: u32,
        len: usize,
        write: bool,
    ) -> Result<Range<usize>, Exception> {
        let fault = Exception::SegmentationViolation(Offset32(addr));
        if write && segment == SrIndex::CS {
            return Err(fault);
        }
        let start = addr as usize;
        let end = start.checked_add(len).ok_or(fault)?;
        let size = if segment == SrIndex::CS {
            self.code().len()
        } else {
            self.data().len()
        };
        if end > size {
            return Err(fault);
        }
        Ok(start..end)
    }

    fn load_fpu(&self, op: FpuMinorUop, format: u8) -> Result<(FpuValue, u16), Exception> {
        let range = self.fpu_range(op, width(format), false)?;
        let bytes = if op.segment() == SrIndex::CS {
            &self.code()[range]
        } else {
            &self.data()[range]
        };
        codec::decode(bytes, format).map_err(|_| Exception::Unimplemented(Uop::Fpu(op)))
    }

    fn last_fpu_opcode(&self) -> u16 {
        let fpu = &self.state.fpu;
        if !fpu.opcode_pending {
            return fpu.last_opcode;
        }
        // Resolve only on an environment save, not on the arithmetic hot path.
        let Some(bytes) = self.code().get(fpu.instruction_pointer as usize..) else {
            return 0;
        };
        codec::opcode(bytes).unwrap_or(0)
    }

    pub(super) fn execute_fpu(&mut self, op: FpuMinorUop) -> Result<(), Exception> {
        use FpuOp::*;
        match op.op {
            #[cfg(feature = "sse")]
            Simd => return self.execute_simd(op),
            LoadControl => {
                let range = self.fpu_range(op, 2, false)?;
                let bytes = if op.segment() == SrIndex::CS {
                    &self.code()[range]
                } else {
                    &self.data()[range]
                };
                let cw = u16::from_le_bytes(bytes.try_into().unwrap());
                if cw & 0x3f != 0x3f {
                    return Err(Exception::Unimplemented(Uop::Fpu(op)));
                }
                self.state.fpu.control = cw;
            }
            LoadEnvironment | Restore => {
                let wide = op.arg != 0;
                let full = op.op == Restore;
                let len = (if wide { 28 } else { 14 }) + if full { 80 } else { 0 };
                let range = self.fpu_range(op, len, false)?;
                let bytes = if op.segment() == SrIndex::CS {
                    &self.code()[range]
                } else {
                    &self.data()[range]
                };
                let mut restored = self.state.fpu.clone();
                // Materialize FOP before a 16-bit restore replaces its source IP.
                restored.last_opcode = self.last_fpu_opcode();
                restored.opcode_pending = false;
                if !restored.restore_environment(bytes, wide, full) {
                    return Err(Exception::Unimplemented(Uop::Fpu(op)));
                }
                self.state.fpu = restored;
            }
            StoreEnvironment | Save => {
                let wide = op.arg != 0;
                let full = op.op == Save;
                let len = (if wide { 28 } else { 14 }) + if full { 80 } else { 0 };
                let range = self.fpu_range(op, len, true)?;
                let (bytes, len) = self
                    .state
                    .fpu
                    .environment(wide, full, self.last_fpu_opcode());
                self.data()[range].copy_from_slice(&bytes[..len]);
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
                self.data()[range].copy_from_slice(&value.to_le_bytes());
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
                self.data()[range].copy_from_slice(&bytes[..width(format)]);
                if op.arg & POP != 0 {
                    self.state.fpu.pop();
                }
            }
            ArithmeticMemory => {
                if !self.state.fpu.supports_arithmetic() {
                    return Err(Exception::Unimplemented(Uop::Fpu(op)));
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
                .map_err(|_| Exception::Unimplemented(Uop::Fpu(op)))?,
        }

        // Only successful non-control instructions advance the architectural
        // pointers. Saves and restores must observe the preceding instruction.
        if op.records_instruction() {
            let selector = self.state.sr(SrIndex::CS).sel.0;
            let fpu = &mut self.state.fpu;
            fpu.instruction_pointer = op.source_eip();
            fpu.instruction_selector = selector;
            fpu.opcode_pending = true;
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
impl crate::simd::ume::UmeMemory for HarvardUME {
    fn state(&self) -> &crate::cpu::ProcessorState {
        &self.state
    }
    fn state_mut(&mut self) -> &mut crate::cpu::ProcessorState {
        &mut self.state
    }
    fn unimplemented(op: FpuMinorUop) -> Exception {
        Exception::Unimplemented(Uop::Fpu(op))
    }
    fn range(
        &self,
        segment: SrIndex,
        addr: u32,
        len: usize,
        write: bool,
    ) -> Result<Range<usize>, Exception> {
        self.fpu_range_at(segment, addr, len, write)
    }
    fn read(&self, segment: SrIndex, range: Range<usize>) -> &[u8] {
        if segment == SrIndex::CS {
            &self.code()[range]
        } else {
            &self.data()[range]
        }
    }
    fn write(&mut self, range: Range<usize>, bytes: &[u8]) {
        self.data()[range].copy_from_slice(bytes);
    }
    fn fpu_opcode(&self) -> u16 {
        self.last_fpu_opcode()
    }
}
