//! Integer rounding path for PC=24 and directed rounding. The default nearest
//! PC=53/64 path remains the approximate native-f64 backend.
use super::{
    DENORMAL, FpuValue, INDEFINITE, INVALID, OVERFLOW, PRECISION, Raw80, UNDERFLOW, ZERO_DIVIDE,
};

fn jam(value: u128, shift: u32) -> u128 {
    if shift == 0 {
        value
    } else if shift < 128 {
        (value >> shift) | u128::from(value << (128 - shift) != 0)
    } else {
        u128::from(value != 0)
    }
}

fn signed(significand: u64, exponent: u16, negative: bool) -> Raw80 {
    Raw80 {
        significand,
        sign_exponent: exponent | if negative { 0x8000 } else { 0 },
    }
}

pub(super) fn normalized(value: Raw80) -> (u64, i32) {
    let shift = value.significand.leading_zeros();
    (
        value.significand << shift,
        i32::from(value.exponent().max(1)) - 16383 - shift as i32,
    )
}

// `bits` is normalized with its leading bit at 66 and three rounding bits.
pub(super) fn finish(
    mut bits: u128,
    mut exponent: i32,
    negative: bool,
    cw: u16,
) -> (FpuValue, u16) {
    let precision = match (cw >> 8) & 3 {
        0 => 24,
        2 => 53,
        3 => 64,
        _ => unreachable!(),
    };
    let rc = (cw >> 10) & 3;
    let shift = 67 - precision;
    let increment = |bits: u128| {
        let tail = bits & ((1u128 << shift) - 1);
        let half = 1u128 << (shift - 1);
        match rc {
            0 => tail > half || (tail == half && (bits >> shift) & 1 != 0),
            1 => negative && tail != 0,
            2 => !negative && tail != 0,
            _ => false,
        }
    };
    // Test tininess after precision rounding with an unbounded exponent,
    // before the additional rounding needed to encode an extended subnormal.
    let tiny = exponent < -16382
        && !(exponent == -16383
            && (bits >> shift) + u128::from(increment(bits)) == 1u128 << precision);
    if exponent < -16382 {
        bits = jam(bits, (-16382 - exponent) as u32);
        exponent = -16382;
    }
    let remainder = bits & ((1u128 << shift) - 1);
    let mut rounded = (bits >> shift) + u128::from(increment(bits));
    if rounded >= 1u128 << precision {
        rounded >>= 1;
        exponent += 1;
    }
    let mut flags = if remainder != 0 { PRECISION } else { 0 };
    if exponent > 16383 {
        let infinite = rc == 0 || (rc == 1 && negative) || (rc == 2 && !negative);
        let raw = if infinite {
            signed(1 << 63, 0x7fff, negative)
        } else {
            signed(u64::MAX << (64 - precision), 0x7ffe, negative)
        };
        return (FpuValue::Raw80(raw), OVERFLOW | PRECISION);
    }
    if tiny && remainder != 0 {
        flags |= UNDERFLOW;
    }
    let significand = (rounded << (64 - precision)) as u64;
    let raw = signed(
        significand,
        if significand >> 63 == 0 {
            0
        } else {
            (exponent + 16383) as u16
        },
        negative,
    );
    // Keep a native representation only when expansion back to Raw80 is exact.
    // Precision control reduces the significand, never the extended exponent range.
    if precision == 24 {
        let (bits, conversion_flags) = raw.to_ieee(23, 8, 0);
        if conversion_flags == 0 {
            return (FpuValue::F32(f32::from_bits(bits as u32)), flags);
        }
    } else if precision == 53 {
        let (value, conversion_flags) = raw.to_f64();
        if conversion_flags == 0 {
            return (FpuValue::F64(value), flags);
        }
    }
    (FpuValue::Raw80(raw), flags)
}

pub(crate) fn binary(lhs: FpuValue, rhs: FpuValue, kind: u8, cw: u16) -> (FpuValue, u16) {
    let mut a = lhs.raw();
    let mut b = rhs.raw();
    let mut flags = if a.is_denormal() || b.is_denormal() {
        DENORMAL
    } else {
        0
    };
    if a.is_signaling_nan() || b.is_signaling_nan() {
        flags |= INVALID;
    }
    if a.is_nan() {
        return (FpuValue::Raw80(a.quiet()), flags);
    }
    if b.is_nan() {
        return (FpuValue::Raw80(b.quiet()), flags);
    }
    if matches!(kind, 5 | 7) {
        core::mem::swap(&mut a, &mut b);
    }
    if matches!(kind, 4 | 5) {
        b.sign_exponent ^= 0x8000;
    }
    let negative = a.is_negative() ^ b.is_negative();
    let invalid = || (FpuValue::Raw80(INDEFINITE), (flags & !DENORMAL) | INVALID);
    let infinity = |sign| (FpuValue::Raw80(signed(1 << 63, 0x7fff, sign)), flags);
    let zero = |sign| (FpuValue::Raw80(signed(0, 0, sign)), flags);
    let (bits, exponent, sign) = match kind {
        0 | 4 | 5 => {
            if a.is_infinite() || b.is_infinite() {
                if a.is_infinite() && b.is_infinite() && negative {
                    return invalid();
                }
                return infinity(if a.is_infinite() {
                    a.is_negative()
                } else {
                    b.is_negative()
                });
            }
            if a.is_zero() && b.is_zero() {
                return zero(if negative {
                    (cw >> 10) & 3 == 1
                } else {
                    a.is_negative()
                });
            }
            // Normalize the nonzero operands before aligning. Up to three low
            // bits are exact; farther bits are jammed into a sticky bit.
            let (mut sa, mut ea) = if a.is_zero() {
                (0, -16445)
            } else {
                normalized(a)
            };
            let (mut sb, mut eb) = if b.is_zero() {
                (0, -16445)
            } else {
                normalized(b)
            };
            if ea < eb {
                core::mem::swap(&mut sa, &mut sb);
                core::mem::swap(&mut ea, &mut eb);
                core::mem::swap(&mut a, &mut b);
            }
            let x = u128::from(sa) << 3;
            let y = jam(u128::from(sb) << 3, (ea - eb) as u32);
            let (mut bits, sign) = if negative {
                if x >= y {
                    (x - y, a.is_negative())
                } else {
                    (y - x, b.is_negative())
                }
            } else {
                (x + y, a.is_negative())
            };
            if bits == 0 {
                return zero((cw >> 10) & 3 == 1);
            }
            let leading = 127 - bits.leading_zeros();
            let exponent = ea + leading as i32 - 66;
            if leading > 66 {
                bits = jam(bits, leading - 66);
            } else {
                bits <<= 66 - leading;
            }
            (bits, exponent, sign)
        }
        1 => {
            if (a.is_zero() && b.is_infinite()) || (b.is_zero() && a.is_infinite()) {
                return invalid();
            }
            if a.is_infinite() || b.is_infinite() {
                return infinity(negative);
            }
            if a.is_zero() || b.is_zero() {
                return zero(negative);
            }
            let (sa, ea) = normalized(a);
            let (sb, eb) = normalized(b);
            let product = u128::from(sa) * u128::from(sb);
            let leading = 127 - product.leading_zeros();
            (
                jam(product, leading - 66),
                ea + eb + leading as i32 - 126,
                negative,
            )
        }
        6 | 7 => {
            if (a.is_zero() && b.is_zero()) || (a.is_infinite() && b.is_infinite()) {
                return invalid();
            }
            if a.is_infinite() {
                return infinity(negative);
            }
            if b.is_infinite() {
                return zero(negative);
            }
            if b.is_zero() {
                return (
                    FpuValue::Raw80(signed(1 << 63, 0x7fff, negative)),
                    (flags & !DENORMAL) | ZERO_DIVIDE,
                );
            }
            if a.is_zero() {
                return zero(negative);
            }
            let (sa, ea) = normalized(a);
            let (sb, eb) = normalized(b);
            let denominator = u128::from(sb);
            let exponent = ea - eb - i32::from(sa < sb);
            // Compute the leading 64 quotient bits in one division. Extend
            // the exact remainder by three bits for finish's rounding input.
            let numerator = u128::from(sa) << (63 + u32::from(sa < sb));
            let mut quotient = numerator / denominator;
            let mut remainder = numerator % denominator;
            for _ in 0..3 {
                remainder <<= 1;
                quotient = (quotient << 1) | u128::from(remainder >= denominator);
                if remainder >= denominator {
                    remainder -= denominator;
                }
            }
            (quotient | u128::from(remainder != 0), exponent, negative)
        }
        _ => unreachable!(),
    };
    let (result, rounded_flags) = finish(bits, exponent, sign, cw);
    (result, flags | rounded_flags)
}

pub(crate) fn sqrt(input: FpuValue, cw: u16) -> (FpuValue, u16) {
    let raw = input.raw();
    let flags = if raw.is_denormal() { DENORMAL } else { 0 };
    if raw.is_nan() {
        return (
            FpuValue::Raw80(raw.quiet()),
            flags | if raw.is_signaling_nan() { INVALID } else { 0 },
        );
    }
    if raw.is_zero() {
        return (input, flags);
    }
    if raw.is_negative() {
        return (FpuValue::Raw80(INDEFINITE), INVALID);
    }
    if raw.is_infinite() {
        return (input, flags);
    }
    let (significand, exponent) = normalized(raw);
    // sqrt(significand * 2^(69 + exponent%2)) has 67 significant
    // bits. Its low six bits are zero. Take the integer square root of the
    // high 128 bits, then extend it by the three remaining pairs of zeros.
    let high = u128::from(significand) << (63 + (exponent & 1));
    let mut root = high.isqrt();
    let mut remainder = high - root * root;
    for _ in 0..3 {
        remainder <<= 2;
        let trial = (root << 2) | 1;
        root <<= 1;
        if remainder >= trial {
            remainder -= trial;
            root |= 1;
        }
    }
    let (result, rounded_flags) = finish(
        root | u128::from(remainder != 0),
        exponent.div_euclid(2),
        false,
        cw,
    );
    (result, flags | rounded_flags)
}
