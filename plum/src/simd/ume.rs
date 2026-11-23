//! The MMX/SSE/SSE2 step of HarvardUME and FlatUME, which differ only in how
//! they reach guest memory ([`UmeMemory`]).
//!
//! These engines have no CR0/CR4: EM = TS = 0 and OSFXSR = OSXMMEXCPT = 1,
//! and no pending x87 exception exists (FLDCW and FRSTOR reject unmasking).
//! LDMXCSR and FXRSTOR reject an unmasked MXCSR exception; a numeric form
//! still stops as unimplemented if the host unmasked one through
//! [`SimdState::set_mxcsr`](super::SimdState::set_mxcsr), since #XM is not
//! delivered ([`super::execute`] checks it). Order: alignment (#GP(0) on the
//! offset; segments are flat), the whole operand, then the change.
use core::ops::Range;

use ir86::prelude::SimdOp;

use super::{FXSAVE_WRITTEN, FxImage, Rm, SimdFault};
use crate::cpu::ProcessorState;
use crate::fpu::uop::FpuMinorUop;
use crate::gpr::PartialRegister;
use crate::prelude::SrIndex;
use crate::ume::Exception;

/// Guest memory and state of a UME engine for [`execute`].
pub(crate) trait UmeMemory {
    fn state(&self) -> &ProcessorState;
    fn state_mut(&mut self) -> &mut ProcessorState;
    /// The engine's stop for `op`: a form or value outside the implemented scope.
    fn unimplemented(op: FpuMinorUop) -> Exception;
    /// The host range of `len` bytes at `segment:addr`, or the engine's
    /// fault; `write` for a destination.
    fn range(
        &self,
        segment: SrIndex,
        addr: u32,
        len: usize,
        write: bool,
    ) -> Result<Range<usize>, Exception>;
    /// The bytes of a `range` that [`Self::range`] returned for `segment`.
    fn read(&self, segment: SrIndex, range: Range<usize>) -> &[u8];
    /// Stores `bytes` into a range that [`Self::range`] returned for a write.
    fn write(&mut self, range: Range<usize>, bytes: &[u8]);
    /// FOP for FXSAVE.
    fn fpu_opcode(&self) -> u16;
}

/// One MMX/SSE/SSE2 Uop on HarvardUME or FlatUME.
#[inline]
pub(crate) fn execute<C: UmeMemory>(cpu: &mut C, op: FpuMinorUop) -> Result<(), Exception> {
    let simd_op = op.simd_op();
    let fault = |fault| match fault {
        SimdFault::GeneralProtection => Exception::GeneralProtection,
        SimdFault::Unimplemented => C::unimplemented(op),
    };
    if let Some(addr32) = op.simd_implicit_addr32() {
        // MASKMOVQ/MASKMOVDQU: the whole operand is checked even when the
        // mask selects no byte; only selected bytes are stored.
        let len = if simd_op == SimdOp::MASKMOVQ_Pq_Nq {
            8
        } else {
            16
        };
        let addr = cpu.state().reg(op.address()).e().read();
        let addr = if addr32 { addr } else { addr & 0xffff };
        let range = cpu.range(op.segment(), addr, len, true)?;
        let (data, mask) = super::mask_move(cpu.state_mut(), simd_op, op.simd_reg(), op.simd_rm());
        for i in (0..len).filter(|i| mask & (1 << i) != 0) {
            let at = range.start + i;
            cpu.write(at..at + 1, &data[i..i + 1]);
        }
        return Ok(());
    }
    let (reg, imm) = (op.simd_reg(), op.simd_imm());
    if !op.simd_is_memory() {
        return super::execute(
            cpu.state_mut(),
            simd_op,
            reg,
            Rm::Reg(op.simd_rm()),
            imm,
            false,
        )
        .map_err(fault);
    }
    let len = usize::from(simd_op.mem_size());
    let addr = cpu.state().reg(op.address()).e().read();
    if simd_op.needs_align16() && addr & 15 != 0 {
        return Err(Exception::GeneralProtection);
    }
    let range = cpu.range(op.segment(), addr, len, simd_op.rm_is_dest())?;
    match simd_op {
        SimdOp::FXSAVE_M512 => {
            let image = super::fxsave(cpu.state(), cpu.fpu_opcode());
            let written = range.start..range.start + FXSAVE_WRITTEN;
            cpu.write(written, &image[..FXSAVE_WRITTEN]);
        }
        SimdOp::FXRSTOR_M512 => {
            let image: FxImage = cpu.read(op.segment(), range).try_into().unwrap();
            super::fxrstor(cpu.state_mut(), &image, false).map_err(fault)?;
        }
        // CLFLUSH only needs its byte to be addressable.
        SimdOp::CLFLUSH_Mb => {}
        _ if simd_op.rm_is_dest() => {
            let bytes = super::store(cpu.state_mut(), simd_op, reg);
            cpu.write(range, &bytes[..len]);
        }
        _ => {
            let mut bytes = [0; 16];
            bytes[..len].copy_from_slice(cpu.read(op.segment(), range));
            super::execute(
                cpu.state_mut(),
                simd_op,
                reg,
                Rm::Mem(&bytes[..len]),
                imm,
                false,
            )
            .map_err(fault)?;
        }
    }
    Ok(())
}
