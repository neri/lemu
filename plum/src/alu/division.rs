//! Checked DIV/IDIV values. Callers gather registers and commit only on success.
// Let each engine commit the two scalar results without an Option tuple ABI.
#[inline(always)]
pub(crate) fn divide(dividend: u64, divisor: u32, bits: u32, signed: bool) -> Option<(u32, u32)> {
    if divisor == 0 {
        return None;
    }
    // Keep the i32/u32 division AND remainder paths below. On 32-bit hosts,
    // 64-bit division can require software helpers even when the dividend
    // fits in 32 bits. All 8/16-bit instructions and many 32-bit instructions
    // can use the narrower paths. Do not merge these into unconditional
    // i64/u64 arithmetic just to remove duplication; that can regress performance.
    let result = if signed {
        let dividend = match bits {
            8 => dividend as i16 as i64,
            16 => dividend as i32 as i64,
            _ => dividend as i64,
        };
        let divisor = match bits {
            8 => divisor as i8 as i64,
            16 => divisor as i16 as i64,
            _ => divisor as i32 as i64,
        };
        let (quotient, remainder) = if let Ok(dividend) = i32::try_from(dividend) {
            let divisor = divisor as i32;
            let quotient = dividend.checked_div(divisor)?;
            (quotient as i64, (dividend % divisor) as u32)
        } else {
            let quotient = dividend.checked_div(divisor)?;
            (quotient, (dividend % divisor) as u32)
        };
        let bound = 1i64 << (bits - 1);
        if quotient < -bound || quotient >= bound {
            return None;
        }
        (quotient as u32, remainder)
    } else {
        let (quotient, remainder) = if let Ok(dividend) = u32::try_from(dividend) {
            ((dividend / divisor) as u64, dividend % divisor)
        } else {
            (
                dividend / divisor as u64,
                (dividend % divisor as u64) as u32,
            )
        };
        if quotient >= (1u64 << bits) {
            return None;
        }
        (quotient as u32, remainder)
    };
    Some(result)
}
