//! x87 byte formats; callers validate memory before decoding or encoding.
use super::uop::{BCD, F32, F64, F80, I16, I32, I64, TRUNCATE};
use super::{DENORMAL, FpuValue, INVALID, Raw80, UnsupportedFpu};

#[inline]
pub(crate) fn width(format: u8) -> usize {
    match format {
        F32 | I32 => 4,
        F64 | I64 => 8,
        F80 | BCD => 10,
        I16 => 2,
        _ => unreachable!(),
    }
}

#[inline]
pub(crate) fn decode(bytes: &[u8], format: u8) -> Result<(FpuValue, u16), UnsupportedFpu> {
    let value = match format {
        F32 => FpuValue::F32(f32::from_le_bytes(bytes.try_into().unwrap())),
        F64 => FpuValue::F64(f64::from_le_bytes(bytes.try_into().unwrap())),
        F80 => FpuValue::Raw80(Raw80 {
            significand: u64::from_le_bytes(bytes[..8].try_into().unwrap()),
            sign_exponent: u16::from_le_bytes(bytes[8..].try_into().unwrap()),
        }),
        BCD => crate::fpu::bcd::load(bytes).ok_or(UnsupportedFpu)?,
        I16 => FpuValue::Raw80(Raw80::from_i64(
            i16::from_le_bytes(bytes.try_into().unwrap()) as i64,
        )),
        I32 => FpuValue::Raw80(Raw80::from_i64(
            i32::from_le_bytes(bytes.try_into().unwrap()) as i64,
        )),
        I64 => FpuValue::Raw80(Raw80::from_i64(i64::from_le_bytes(
            bytes.try_into().unwrap(),
        ))),
        _ => unreachable!(),
    };
    let raw = value.raw();
    if !raw.is_supported() {
        return Err(UnsupportedFpu);
    }
    // m80 loads transport the extended format without the m32/m64 conversion.
    if format == F80 {
        return Ok((value, 0));
    }
    let denormal = match value {
        FpuValue::F32(v) => v.is_subnormal(),
        FpuValue::F64(v) => v.is_subnormal(),
        _ => false,
    };
    let flags = if denormal { DENORMAL } else { 0 };
    if raw.is_signaling_nan() {
        Ok((FpuValue::Raw80(raw.quiet()), flags | INVALID))
    } else {
        Ok((value, flags))
    }
}

#[inline]
pub(crate) fn encode(value: Raw80, arg: u8, rounding: u16) -> ([u8; 10], u16) {
    let format = arg & 7;
    let mut bytes = [0u8; 10];
    let flags = if format == BCD {
        let (packed, flags) = crate::fpu::bcd::store(value, rounding);
        bytes = packed;
        flags
    } else if format == F80 {
        bytes[..8].copy_from_slice(&value.significand.to_le_bytes());
        bytes[8..].copy_from_slice(&value.sign_exponent.to_le_bytes());
        0
    } else {
        let rc = if arg & TRUNCATE != 0 { 3 } else { rounding };
        let (bits, flags) = match format {
            F32 => value.to_ieee(23, 8, rounding),
            F64 => value.to_ieee(52, 11, rounding),
            I16 => value.to_integer(16, rc),
            I32 => value.to_integer(32, rc),
            I64 => value.to_integer(64, rc),
            _ => unreachable!(),
        };
        let size = width(format);
        bytes[..size].copy_from_slice(&bits.to_le_bytes()[..size]);
        flags
    };
    (bytes, flags)
}

/// Decode FOP from the original instruction bytes, including legacy prefixes.
#[inline]
pub(crate) fn opcode(bytes: &[u8]) -> Option<u16> {
    let index = bytes.iter().position(|b| {
        !matches!(
            b,
            0x26 | 0x2e | 0x36 | 0x3e | 0x64..=0x67 | 0xf0 | 0xf2 | 0xf3
        )
    })?;
    match bytes.get(index..index + 2)? {
        [opcode @ 0xd8..=0xdf, modrm] => Some(((u16::from(*opcode) & 7) << 8) | u16::from(*modrm)),
        _ => None,
    }
}
