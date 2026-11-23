//! MMX registers on the x87 physical registers R0-R7.
//!
//! MMn is the significand of physical register Rn (not ST(n)). Every MMX
//! instruction but EMMS sets TOP to 0 and every tag to valid; a write also sets
//! the sign and exponent of Rn to all ones (SDM Vol. 1, 9.5). FCW, the other
//! FSW bits, FIP, FDP and FOP do not change, so MMX does not use the x87
//! instruction record (`records_instruction`).
use crate::fpu::{FpuState, FpuValue, Raw80};

/// TOP = 0 and all tags valid: the state after any MMX instruction but EMMS.
#[inline]
pub(super) fn enter(fpu: &mut FpuState) {
    fpu.top = 0;
    fpu.tags = 0;
}

/// EMMS: every tag empty.
#[inline]
pub(super) fn emms(fpu: &mut FpuState) {
    fpu.tags = 0xffff;
}

/// MMn, the low 64 bits of physical register Rn.
#[inline]
pub(super) fn read(fpu: &FpuState, index: usize) -> u64 {
    fpu.registers[index].raw().significand
}

/// Writes MMn: Rn becomes `value` with sign and exponent all ones, kept as
/// Raw80 so the 64 bits survive exactly.
#[inline]
pub(super) fn write(fpu: &mut FpuState, index: usize, value: u64) {
    fpu.registers[index] = FpuValue::Raw80(Raw80 {
        significand: value,
        sign_exponent: 0xffff,
    });
}
