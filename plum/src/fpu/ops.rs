//! Numerical operations for the approximate x87 backend.
use super::{DENORMAL, FpuValue, INDEFINITE, INVALID, OVERFLOW, PRECISION, UNDERFLOW, ZERO_DIVIDE};

#[inline]
pub(crate) fn arithmetic(lhs: FpuValue, rhs: FpuValue, kind: u8, cw: u16) -> (FpuValue, u16) {
    if cw & 0x0c00 != 0 || cw & 0x0300 == 0 {
        return crate::fpu::arithmetic::binary(lhs, rhs, kind, cw);
    }
    // The common F64/F64 path classifies the native bits directly. It never
    // materializes Raw80 between consecutive arithmetic instructions.
    let mut flags = if lhs.is_denormal() || rhs.is_denormal() {
        DENORMAL
    } else {
        0
    };
    if lhs.is_signaling_nan() || rhs.is_signaling_nan() {
        flags |= INVALID;
    }
    if lhs.is_nan() {
        return (FpuValue::Raw80(lhs.raw().quiet()), flags);
    }
    if rhs.is_nan() {
        return (FpuValue::Raw80(rhs.raw().quiet()), flags);
    }
    let (mut a, af) = lhs.approximate();
    let (mut b, bf) = rhs.approximate();
    flags |= af | bf;
    if matches!(kind, 5 | 7) {
        core::mem::swap(&mut a, &mut b);
    }
    let result = match kind {
        0 => a + b,
        1 => a * b,
        4 | 5 => a - b,
        6 | 7 => a / b,
        _ => unreachable!(),
    };
    if result.is_nan() {
        flags |= INVALID;
        return (FpuValue::Raw80(INDEFINITE), flags);
    }
    if matches!(kind, 6 | 7) && b == 0.0 && a != 0.0 && a.is_finite() {
        flags |= ZERO_DIVIDE;
    }
    if result.is_infinite() && a.is_finite() && b.is_finite() && flags & ZERO_DIVIDE == 0 {
        flags |= OVERFLOW | PRECISION;
    }
    if result != 0.0 && result.is_subnormal() {
        flags |= UNDERFLOW;
    }
    // Ordinary host operations do not expose all x87 exception flags. This
    // backend deliberately promises f64 results, not exact x87 PE/UE/C1.
    (FpuValue::F64(result), flags)
}

#[inline]
pub(crate) fn square_root(input: FpuValue, cw: u16) -> (FpuValue, u16) {
    if cw & 0x0c00 != 0 || cw & 0x0300 == 0 {
        return crate::fpu::arithmetic::sqrt(input, cw);
    }
    let (value, mut flags) = input.approximate();
    let result = if input.is_nan() {
        if input.is_signaling_nan() {
            flags |= INVALID;
        }
        FpuValue::Raw80(input.raw().quiet())
    } else if input.is_negative_nonzero() {
        flags |= INVALID;
        FpuValue::Raw80(INDEFINITE)
    } else {
        FpuValue::F64(libm::sqrt(value))
    };
    if input.is_denormal() {
        flags |= DENORMAL;
    }
    (result, flags)
}
