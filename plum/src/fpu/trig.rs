//! Nearest-only libm approximation. PC does not narrow transcendental results.
//! This does not reproduce the x87 internal pi reduction or its rounding flags.
use super::{DENORMAL, FpuValue, INDEFINITE, INVALID, PRECISION, Raw80, UNDERFLOW};

pub(crate) enum UnaryResult {
    OutOfRange,
    Value {
        first: FpuValue,
        pushed: Option<FpuValue>,
        flags: u16,
    },
}

pub(crate) fn unary(input: FpuValue, kind: u8) -> UnaryResult {
    let raw = input.raw();
    let pair = kind >= 2;
    let finish = |first, second, flags| UnaryResult::Value {
        first,
        pushed: pair.then_some(second),
        flags,
    };
    if raw.is_nan() || raw.is_infinite() {
        let (value, flags) = if raw.is_nan() {
            (
                FpuValue::Raw80(raw.quiet()),
                if raw.is_signaling_nan() { INVALID } else { 0 },
            )
        } else {
            (FpuValue::Raw80(INDEFINITE), INVALID)
        };
        // Masked invalid and NaN propagation affect both destinations, including
        // FPTAN's normally constant new ST0.
        return finish(value, value, flags);
    }
    // Inspect the original value: rounding a value just below 2^63 to f64
    // would incorrectly mark it out of range and suppress the push.
    if raw.exponent() >= 16383 + 63 {
        return UnaryResult::OutOfRange;
    }
    let one = FpuValue::F32(1.0);
    if raw.is_zero() {
        return finish(if kind == 1 { one } else { input }, one, 0);
    }
    let mut flags = PRECISION | if raw.is_denormal() { DENORMAL } else { 0 };
    // sin(x) and tan(x) approximate x here. Preserve extended tiny values
    // instead of underflowing them during conversion to f64.
    if raw.exponent() < 16383 - 27 {
        if raw.is_denormal() && kind != 1 {
            flags |= UNDERFLOW;
        }
        return finish(if kind == 1 { one } else { input }, one, flags);
    }
    let (value, conversion_flags) = input.approximate();
    flags |= conversion_flags;
    let (first, second) = if conversion_flags & PRECISION != 0 {
        // Raw80 has up to eleven bits beyond f64. Preserve their phase by
        // evaluating hi + lo rather than discarding them at large angles.
        let (residual, _) = super::arithmetic::binary(input, FpuValue::F64(value), 4, 0x037f);
        let (low, _) = residual.approximate();
        let (sh, ch) = libm::sincos(value);
        let (sl, cl) = libm::sincos(low);
        let sine = libm::fma(sh, cl, ch * sl);
        let cosine = libm::fma(ch, cl, -sh * sl);
        match kind {
            0 => (sine, 1.0),
            1 => (cosine, 1.0),
            2 => (sine, cosine),
            3 => (sine / cosine, 1.0),
            _ => unreachable!(),
        }
    } else {
        match kind {
            0 => (libm::sin(value), 1.0),
            1 => (libm::cos(value), 1.0),
            2 => libm::sincos(value),
            3 => (libm::tan(value), 1.0),
            _ => unreachable!(),
        }
    };
    finish(FpuValue::F64(first), FpuValue::F64(second), flags)
}

fn normalized(raw: Raw80) -> (f64, i32) {
    let shift = raw.significand.leading_zeros();
    let significand = raw.significand << shift;
    let mantissa = significand as f64 * (1.0 / 9223372036854775808.0);
    let exponent = i32::from(raw.exponent().max(1)) - 16383 - shift as i32;
    (
        if raw.is_negative() {
            -mantissa
        } else {
            mantissa
        },
        exponent,
    )
}

pub(crate) fn atan(y: FpuValue, x: FpuValue) -> (FpuValue, u16) {
    let yr = y.raw();
    let xr = x.raw();
    let mut flags = if yr.is_denormal() || xr.is_denormal() {
        DENORMAL
    } else {
        0
    };
    if let Some(result) = super::value::propagate_nan(xr, yr) {
        return result;
    }
    if yr.is_zero() || xr.is_zero() || yr.is_infinite() || xr.is_infinite() {
        let classified = |raw: Raw80| {
            let value = if raw.is_zero() {
                0.0f64
            } else if raw.is_infinite() {
                f64::INFINITY
            } else {
                1.0
            };
            if raw.is_negative() { -value } else { value }
        };
        let result = libm::atan2(classified(yr), classified(xr));
        if result != 0.0 {
            flags |= PRECISION;
        }
        return (FpuValue::F64(result), flags);
    }
    let (ym, ye) = normalized(yr);
    let (xm, xe) = normalized(xr);
    if !xr.is_negative() && ye - xe < -27 {
        // atan(r) approximates r. An extended division preserves small angles
        // outside the host exponent range and avoids dividing inf by inf.
        let (result, division_flags) = super::arithmetic::binary(y, x, 6, 0x037f);
        flags |= division_flags | PRECISION;
        if result.raw().exponent() == 0 {
            flags |= UNDERFLOW;
        }
        return (result, flags);
    }
    let common = ye.max(xe);
    let result = libm::atan2(libm::scalbn(ym, ye - common), libm::scalbn(xm, xe - common));
    (FpuValue::F64(result), flags | PRECISION)
}
