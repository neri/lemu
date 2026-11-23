//! Nearest-only approximations, preserving the extended range around libm.
use super::{DENORMAL, FpuValue, INDEFINITE, INVALID, PRECISION, Raw80, ZERO_DIVIDE};

pub(crate) fn exp_in_range(raw: Raw80) -> bool {
    raw.is_nan()
        || raw.exponent() < 16383
        || (raw.exponent() == 16383 && raw.significand == 1 << 63)
}
pub(crate) fn log1p_in_range(raw: Raw80) -> bool {
    raw.is_nan()
        || raw.exponent() < 0x3ffd
        || (raw.exponent() == 0x3ffd && raw.significand <= 0x95f619980c4336f7)
}

// Multiply a finite coefficient by y*2^shift, combining exponents before
// rounding so a tiny intermediate is not lost before multiplication by large y.
fn multiply_scaled(coefficient: f64, shift: i32, y: FpuValue) -> (FpuValue, u16) {
    let raw = y.raw();
    let (sig, exponent) = super::arithmetic::normalized(raw);
    let normalized = FpuValue::Raw80(Raw80 {
        significand: sig,
        sign_exponent: 16383 | (raw.sign_exponent & 0x8000),
    });
    let (product, flags) =
        super::arithmetic::binary(FpuValue::F64(coefficient), normalized, 1, 0x037f);
    let (result, scaled_flags) =
        super::exponent::scale(product, FpuValue::F64(f64::from(exponent + shift)), 0x037f);
    (result, flags | scaled_flags)
}

pub(crate) fn exp2m1(input: FpuValue) -> (FpuValue, u16) {
    let raw = input.raw();
    if raw.is_nan() {
        return (
            FpuValue::Raw80(raw.quiet()),
            if raw.is_signaling_nan() { INVALID } else { 0 },
        );
    }
    if raw.is_zero() {
        return (input, 0);
    }
    let flags = if raw.is_denormal() { DENORMAL } else { 0 };
    if raw.exponent() == 16383 {
        return (
            FpuValue::F64(if raw.is_negative() { -0.5 } else { 1.0 }),
            flags,
        );
    }
    if raw.exponent() < 16383 - 27 {
        let (result, rounded_flags) = multiply_scaled(core::f64::consts::LN_2, 0, input);
        return (result, flags | rounded_flags | PRECISION);
    }
    let (x, converted) = input.approximate();
    (
        FpuValue::F64(libm::expm1(x * core::f64::consts::LN_2)),
        flags | converted | PRECISION,
    )
}

pub(crate) fn log_multiply(x: FpuValue, y: FpuValue, plus_one: bool) -> (FpuValue, u16) {
    let a = x.raw();
    let b = y.raw();
    if let Some(result) = super::value::propagate_nan(a, b) {
        return result;
    }
    let flags = if a.is_denormal() || b.is_denormal() {
        DENORMAL
    } else {
        0
    };
    if !plus_one && a.is_negative() && !a.is_zero() {
        return (FpuValue::Raw80(INDEFINITE), INVALID);
    }
    // A zero logarithm still participates in multiplication: 0*infinity is invalid.
    let log_zero = if plus_one {
        a.is_zero()
    } else {
        a == Raw80::from_i64(1)
    };
    if log_zero {
        if b.is_infinite() {
            return (FpuValue::Raw80(INDEFINITE), INVALID);
        }
        let negative = b.is_negative() ^ (plus_one && a.is_negative());
        return (
            FpuValue::Raw80(Raw80::from_i64(0).with_sign(negative)),
            flags,
        );
    }
    if !plus_one && (a.is_zero() || a.is_infinite()) {
        if b.is_zero() {
            return (FpuValue::Raw80(INDEFINITE), INVALID);
        }
        let negative = b.is_negative() ^ a.is_zero();
        return (
            FpuValue::Raw80(Raw80 {
                significand: 1 << 63,
                sign_exponent: 0x7fff | if negative { 0x8000 } else { 0 },
            }),
            if a.is_zero() && !b.is_infinite() {
                ZERO_DIVIDE
            } else {
                flags
            },
        );
    }
    let negative_log = if plus_one {
        a.is_negative()
    } else {
        a.exponent() < 16383
    };
    if b.is_zero() || b.is_infinite() {
        return (
            FpuValue::Raw80(b.with_sign(b.is_negative() ^ negative_log)),
            flags,
        );
    }
    if plus_one && a.exponent() < 16383 - 27 {
        let (sig, exponent) = super::arithmetic::normalized(a);
        let mantissa =
            sig as f64 / 9223372036854775808.0 * if a.is_negative() { -1.0 } else { 1.0 };
        let (result, rounded_flags) =
            multiply_scaled(mantissa * core::f64::consts::LOG2_E, exponent, y);
        return (result, flags | rounded_flags | PRECISION);
    }
    let logarithm = if plus_one {
        libm::log1p(x.approximate().0) * core::f64::consts::LOG2_E
    } else if matches!(a.exponent(), 0x3ffe | 0x3fff) {
        let (delta, _) = super::arithmetic::binary(x, FpuValue::F64(1.0), 4, 0x037f);
        libm::log1p(delta.approximate().0) * core::f64::consts::LOG2_E
    } else {
        let (sig, exponent) = super::arithmetic::normalized(a);
        libm::log2(sig as f64 / 9223372036854775808.0) + f64::from(exponent)
    };
    let (result, rounded_flags) = super::arithmetic::binary(y, FpuValue::F64(logarithm), 1, 0x037f);
    (
        result,
        flags
            | rounded_flags
            | if plus_one || a.significand != 1 << 63 {
                PRECISION
            } else {
                0
            },
    )
}
