//! Exponent operations retain the extended significand and exponent range.
use super::{DENORMAL, FpuValue, INDEFINITE, INVALID, Raw80, ZERO_DIVIDE};

pub(crate) fn scale(input: FpuValue, power: FpuValue, cw: u16) -> (FpuValue, u16) {
    let a = input.raw();
    let b = power.raw();
    if let Some(result) = super::value::propagate_nan(a, b) {
        return result;
    }
    let flags = if a.is_denormal() || b.is_denormal() {
        DENORMAL
    } else {
        0
    };
    if b.is_infinite() {
        if (a.is_infinite() && b.is_negative()) || (a.is_zero() && !b.is_negative()) {
            return (FpuValue::Raw80(INDEFINITE), INVALID);
        }
        return (
            FpuValue::Raw80(Raw80 {
                significand: if b.is_negative() { 0 } else { 1 << 63 },
                sign_exponent: (if b.is_negative() { 0 } else { 0x7fff })
                    | (a.sign_exponent & 0x8000),
            }),
            flags,
        );
    }
    if a.is_zero() || a.is_infinite() {
        return (input, flags);
    }
    // Finite powers beyond this bound already force the result out of the
    // extended range. Clamp before integer conversion, without a spurious IE.
    let power = if b.exponent() >= 16383 + 16 {
        if b.is_negative() { -65536 } else { 65536 }
    } else {
        b.to_integer(32, 3).0 as i32
    };
    let (significand, exponent) = super::arithmetic::normalized(a);
    // FSCALE ignores PC; RC only matters when the extended result over/underflows.
    let (result, rounded_flags) = super::arithmetic::finish(
        u128::from(significand) << 3,
        exponent + power,
        a.is_negative(),
        (cw & !0x0300) | 0x0300,
    );
    (result, flags | rounded_flags)
}

/// Returns the old ST0 destination (exponent), pushed significand, and flags.
pub(crate) fn extract(input: FpuValue) -> (FpuValue, FpuValue, u16) {
    let raw = input.raw();
    if raw.is_nan() {
        let quiet = FpuValue::Raw80(raw.quiet());
        return (
            quiet,
            quiet,
            if raw.is_signaling_nan() { INVALID } else { 0 },
        );
    }
    if raw.is_zero() {
        return (FpuValue::F64(f64::NEG_INFINITY), input, ZERO_DIVIDE);
    }
    if raw.is_infinite() {
        return (FpuValue::F64(f64::INFINITY), input, 0);
    }
    let (significand, exponent) = super::arithmetic::normalized(raw);
    (
        FpuValue::Raw80(Raw80::from_i64(i64::from(exponent))),
        FpuValue::Raw80(Raw80 {
            significand,
            sign_exponent: 16383 | (raw.sign_exponent & 0x8000),
        }),
        if raw.is_denormal() { DENORMAL } else { 0 },
    )
}
