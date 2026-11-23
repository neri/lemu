//! MMX, SSE and SSE2 forms.
//!
//! The decoder reads every form in src/simd.csv, whatever the CPU it emulates
//! reports in CPUID; the engines decide whether to execute it. `IrOp::UD` is
//! left for encodings that no MMX/SSE/SSE2 CPU defines.

use crate::_prelude_::*;

#[path = "_generated/simd.rs"]
mod generated;

pub use generated::SimdOp;
pub(crate) use generated::{simd_lookup, simd_shape};

/// XMM register index (XMM0-XMM7)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum XmmIndex {
    XMM0 = 0,
    XMM1 = 1,
    XMM2 = 2,
    XMM3 = 3,
    XMM4 = 4,
    XMM5 = 5,
    XMM6 = 6,
    XMM7 = 7,
}

impl XmmIndex {
    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => Self::XMM0,
            Index3::I001 => Self::XMM1,
            Index3::I010 => Self::XMM2,
            Index3::I011 => Self::XMM3,
            Index3::I100 => Self::XMM4,
            Index3::I101 => Self::XMM5,
            Index3::I110 => Self::XMM6,
            Index3::I111 => Self::XMM7,
        }
    }

    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// MMX register index (MM0-MM7), aliasing the x87 physical registers R0-R7
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MmIndex {
    MM0 = 0,
    MM1 = 1,
    MM2 = 2,
    MM3 = 3,
    MM4 = 4,
    MM5 = 5,
    MM6 = 6,
    MM7 = 7,
}

impl MmIndex {
    #[inline]
    pub const fn from_idx3(index: Index3) -> Self {
        match index {
            Index3::I000 => Self::MM0,
            Index3::I001 => Self::MM1,
            Index3::I010 => Self::MM2,
            Index3::I011 => Self::MM3,
            Index3::I100 => Self::MM4,
            Index3::I101 => Self::MM5,
            Index3::I110 => Self::MM6,
            Index3::I111 => Self::MM7,
        }
    }

    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Register class of a ModR/M operand of a [`SimdOp`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdRegKind {
    /// No register operand (opcode extension, or memory only).
    None,
    Xmm,
    Mm,
    /// 32-bit general-purpose register, in 16-bit and 32-bit modes alike.
    Gpr,
}

/// A register operand of a [`SimdOp`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdReg {
    /// The field is an opcode extension, or the form has no such operand.
    None,
    Xmm(XmmIndex),
    Mm(MmIndex),
    Gpr(GprIndex32),
}

impl SimdReg {
    #[inline]
    pub const fn new(kind: SimdRegKind, index: Index3) -> Self {
        match kind {
            SimdRegKind::None => Self::None,
            SimdRegKind::Xmm => Self::Xmm(XmmIndex::from_idx3(index)),
            SimdRegKind::Mm => Self::Mm(MmIndex::from_idx3(index)),
            SimdRegKind::Gpr => Self::Gpr(GprIndex32::from_idx3(index)),
        }
    }

    /// The register number (0-7), or 0 for `None`.
    #[inline]
    pub const fn index(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Xmm(r) => r as u8,
            Self::Mm(r) => r as u8,
            Self::Gpr(r) => r as u8,
        }
    }
}

/// Exception class of a [`SimdOp`]: which of CR0.EM, CR0.TS, CR4.OSFXSR and a
/// pending x87 exception the form checks (docs/DESIGN.md, 9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdClass {
    /// EM: #UD, TS: #NM, OSFXSR=0: #UD; ignores pending x87 exceptions.
    Xmm,
    /// EM: #UD, TS: #NM, OSFXSR=0: #UD, pending x87 exception: #MF.
    MmXmm,
    /// EM: #UD, TS: #NM, pending x87 exception: #MF; ignores OSFXSR.
    Mm,
    /// FXSAVE/FXRSTOR. EM: #NM, TS: #NM.
    FxState,
    /// CLFLUSH, fences, MOVNTI: none of the checks.
    NoCheck,
}

/// CPUID leaf 1 EDX feature that defines a [`SimdOp`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdIsa {
    Mmx,
    Sse,
    Sse2,
    Fxsr,
    Clfsh,
}

/// Mandatory prefix of an SSE opcode (docs/DESIGN.md, 9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SimdPrefix {
    None,
    P66,
    F3,
    F2,
}

/// ModR/M and imm8 presence of an opcode in the SIMD ranges.
pub(crate) struct SimdShape {
    pub has_modrm: bool,
    pub has_imm8: bool,
}
