//! Lazy Emulator Plum

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod alu;
pub mod cpu;
#[cfg(feature = "flag-elision-analysis")]
mod elision;
pub mod flags;
pub mod flat;
pub mod gpr;
pub mod harvard;
mod string_ops;
pub mod types;
pub mod ume;

#[cfg(feature = "x87")]
pub mod fpu;
#[cfg(feature = "uop-profile")]
mod profile_support;
#[cfg(feature = "sse")]
pub mod simd;

#[cfg(test)]
mod tests;

#[rustfmt::skip]
//                                           |--+---+---|
pub const MANUFACTURER_STRING: &[u8; 12] = b"LazyEmulator";

pub mod prelude {
    pub use ir86::prelude::*;

    pub use crate::cpu::*;
    pub use crate::flags::*;
    pub use crate::types::*;
    pub use crate::ume::*;
}

#[allow(unused_imports)]
pub(crate) mod _prelude_ {
    pub use alloc::boxed::Box;
    pub use alloc::collections::BTreeMap;
    pub use alloc::string::String;
    pub use alloc::sync::Arc;
    pub use alloc::vec::Vec;

    pub use crate::prelude::*;

    /// Copies `s` into a NUL-padded `[u8; N]`, failing at compile time if it does not fit.
    /// At least one trailing NUL is kept, as CPUID brand strings require.
    pub const fn nul_padded<const N: usize>(s: &str) -> [u8; N] {
        let bytes = s.as_bytes();
        assert!(bytes.len() < N, "string too long");
        let mut out = [0u8; N];
        let mut i = 0;
        while i < bytes.len() {
            out[i] = bytes[i];
            i += 1;
        }
        out
    }
}
