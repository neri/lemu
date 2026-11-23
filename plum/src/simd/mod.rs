//! SSE/SSE2 state and execution shared by the three engines (`sse` feature).
//!
//! The engines resolve addresses, check alignment, access memory and report
//! faults; this module holds the XMM registers and MXCSR and computes results.
//! MMX registers alias the x87 registers and live in [`crate::fpu::FpuState`].

mod exec;
mod float;
mod fxsave;
mod lanes;
mod mmx;
pub(crate) mod ume;

#[allow(unused_imports)]
pub(crate) use exec::{Rm, SimdFault, execute, mask_move, numeric, store};
pub(crate) use fxsave::{FXSAVE_WRITTEN, FxImage, fxrstor, fxsave};

/// One XMM register, little-endian bytes. The byte-array API has alignment 1;
/// registers stored in `SimdState` are guaranteed to be 16-byte aligned.
pub type Xmm = [u8; 16];

/// MXCSR at reset: all exceptions masked, round to nearest, FTZ/DAZ off.
pub const MXCSR_RESET: u32 = MXCSR_EXCEPTION_MASKS;
/// MXCSR bits PLUM implements, DAZ included. FXSAVE writes this as
/// MXCSR_MASK; LDMXCSR and FXRSTOR raise #GP(0) for any other bit.
pub const MXCSR_MASK: u32 = 0xffff;

/// MXCSR IM, DM, ZM, OM, UM and PM: every exception masked when all are set.
pub(crate) const MXCSR_EXCEPTION_MASKS: u32 = 0x1f80;

/// XMM0-XMM7 and MXCSR.
///
/// Keep the register bank first and 16-byte aligned even when preceding
/// fields in `ProcessorState` change. Each register has a 16-byte stride.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, align(16))]
pub struct SimdState {
    pub(crate) xmm: [Xmm; 8],
    pub(crate) mxcsr: u32,
}

const _: () = {
    assert!(core::mem::align_of::<SimdState>() == 16);
    assert!(core::mem::offset_of!(SimdState, xmm) == 0);
    assert!(core::mem::size_of::<Xmm>() == 16);
};

impl Default for SimdState {
    fn default() -> Self {
        Self {
            xmm: [[0; 16]; 8],
            mxcsr: MXCSR_RESET,
        }
    }
}

impl SimdState {
    /// Returns XMM`index` (0-7).
    #[inline]
    pub fn xmm(&self, index: usize) -> &Xmm {
        &self.xmm[index]
    }

    /// Returns XMM`index` (0-7) for modification.
    #[inline]
    pub fn xmm_mut(&mut self, index: usize) -> &mut Xmm {
        &mut self.xmm[index]
    }

    #[inline]
    pub fn mxcsr(&self) -> u32 {
        self.mxcsr
    }

    /// Whether MXCSR masks every numeric exception.
    #[inline]
    pub(crate) fn all_masked(&self) -> bool {
        self.mxcsr & MXCSR_EXCEPTION_MASKS == MXCSR_EXCEPTION_MASKS
    }

    /// Sets MXCSR, or returns `false` (state unchanged) when a bit outside
    /// [`MXCSR_MASK`] is set: the caller raises #GP(0).
    #[inline]
    pub fn set_mxcsr(&mut self, value: u32) -> bool {
        if value & !MXCSR_MASK != 0 {
            return false;
        }
        self.mxcsr = value;
        true
    }
}
