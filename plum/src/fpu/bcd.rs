//! Packed 18-digit BCD transport without narrowing through f64.
use super::{FpuValue, INVALID, Raw80};

pub(crate) fn load(bytes: &[u8]) -> Option<FpuValue> {
    let mut magnitude = 0i64;
    for &byte in bytes[..9].iter().rev() {
        let hi = byte >> 4;
        let lo = byte & 15;
        if hi > 9 || lo > 9 {
            return None;
        } // Non-BCD digits have no specified numerical value.
        magnitude = magnitude * 100 + i64::from(hi) * 10 + i64::from(lo);
    }
    Some(FpuValue::Raw80(
        Raw80::from_i64(magnitude).with_sign(bytes[9] & 0x80 != 0),
    ))
}

pub(crate) fn store(value: Raw80, rc: u16) -> ([u8; 10], u16) {
    let (integer, flags) = value.to_integer(64, rc);
    let mut magnitude = (integer as i64).unsigned_abs();
    if flags & INVALID != 0 || magnitude >= 1_000_000_000_000_000_000 {
        return ([0, 0, 0, 0, 0, 0, 0, 0xc0, 0xff, 0xff], INVALID);
    }
    let mut bytes = [0; 10];
    for byte in &mut bytes[..9] {
        let pair = (magnitude % 100) as u8;
        *byte = ((pair / 10) << 4) | (pair % 10);
        magnitude /= 100;
    }
    bytes[9] = if value.is_negative() { 0x80 } else { 0 };
    (bytes, flags)
}
