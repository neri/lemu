//! Exact extended remainder, including architecturally visible partial reduction.
use super::{DENORMAL, FpuValue, INDEFINITE, INVALID, Raw80};

fn pack(magnitude: u64, exponent: i32, negative: bool) -> FpuValue {
    if magnitude == 0 {
        return FpuValue::Raw80(Raw80::from_i64(0).with_sign(negative));
    }
    let shift = magnitude.leading_zeros();
    super::arithmetic::finish(
        u128::from(magnitude << shift) << 3,
        exponent - shift as i32,
        negative,
        0x037f,
    )
    .0
}

/// Returns value, numeric exceptions, and C0/C1/C2/C3. PC and RC do not apply.
pub(crate) fn remainder(input: FpuValue, modulus: FpuValue, nearest: bool) -> (FpuValue, u16, u16) {
    let a = input.raw();
    let b = modulus.raw();
    if let Some((value, flags)) = super::value::propagate_nan(a, b) {
        return (value, flags, 0);
    }
    if a.is_infinite() || b.is_zero() {
        return (FpuValue::Raw80(INDEFINITE), INVALID, 0);
    }
    let flags = if a.is_denormal() || b.is_denormal() {
        DENORMAL
    } else {
        0
    };
    if a.is_zero() || b.is_infinite() {
        return (input, flags, 0);
    }
    let (sa, ea) = super::arithmetic::normalized(a);
    let (sb, eb) = super::arithmetic::normalized(b);
    let difference = ea - eb;
    if difference < 0 {
        if nearest && difference == -1 && sa > sb {
            return (
                pack(
                    (u128::from(sb) * 2 - u128::from(sa)) as u64,
                    ea,
                    !a.is_negative(),
                ),
                flags,
                0x200,
            );
        }
        return (input, flags, 0);
    }
    let partial = difference >= 64;
    // N=32+(D mod 32) is within the specified 32..63 range and preserves
    // quotient bits when guest code repeats until C2 clears.
    let shift = if partial {
        32 + difference % 32
    } else {
        difference
    };
    let numerator = u128::from(sa) << shift;
    let denominator = u128::from(sb);
    let mut quotient = numerator / denominator;
    let mut residual = numerator % denominator;
    let mut negative = a.is_negative();
    if !partial
        && nearest
        && (residual * 2 > denominator || (residual * 2 == denominator && quotient & 1 != 0))
    {
        quotient += 1;
        residual = denominator - residual;
        if residual != 0 {
            negative = !negative;
        }
    }
    let condition = if partial {
        0x400
    } else {
        ((quotient as u16 & 4) << 6) | ((quotient as u16 & 2) << 13) | ((quotient as u16 & 1) << 9)
    };
    (
        pack(residual as u64, ea - shift, negative),
        flags,
        condition,
    )
}
