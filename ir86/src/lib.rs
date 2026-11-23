//! Intermediate representation for x86 instructions.
//!
//! This library decodes x86 instructions and converts them into an intermediate representation.
//!
//! The IR is normalized, so it may not be possible to reconstruct the original instruction exactly from the IR.
//! Therefore, it is not useful for applications where the exact encoding of the original instruction is important.
//!

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod decoder;
pub mod encoding;
pub mod fetch;
#[path = "_generated/ir.rs"]
pub mod ir;
pub mod simd;
pub mod types;

pub mod prelude {
    pub use crate::decoder::*;
    pub use crate::encoding::*;
    pub use crate::fetch::*;
    pub use crate::ir::*;
    pub use crate::simd::{MmIndex, SimdClass, SimdIsa, SimdOp, SimdReg, SimdRegKind, XmmIndex};
    pub use crate::types::*;

    pub mod registers {
        pub use super::GprIndex8::*;
        pub use super::GprIndex16::*;
        pub use super::GprIndex32::*;
        pub use super::SrIndex::*;
    }
}

pub(crate) mod _prelude_ {
    pub use crate::prelude::registers::*;
    pub use crate::prelude::*;
}

const _: () = assert!(core::mem::size_of::<ir::IrOp>() == 16);
