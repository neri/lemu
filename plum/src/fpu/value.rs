//! Lossless register transport, independent of the approximate arithmetic backend.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raw80 {
    pub significand: u64,
    pub sign_exponent: u16,
}

pub(crate) const INVALID: u16 = 1;
pub(crate) const DENORMAL: u16 = 2;
pub(crate) const ZERO_DIVIDE: u16 = 4;
pub(crate) const OVERFLOW: u16 = 8;
pub(crate) const UNDERFLOW: u16 = 16;
pub(crate) const PRECISION: u16 = 32;
pub(crate) const INDEFINITE: Raw80 = Raw80 {
    significand: 0xc000_0000_0000_0000,
    sign_exponent: 0xffff,
};

/// The active representation is authoritative; unused previous values never survive arithmetic.
#[derive(Debug, Clone, Copy, PartialEq)]
// Discriminant zero is a valid Raw80 value, preserving ProcessorState's
// existing zero-initialization invariant before its explicit reset.
#[repr(u8)]
pub enum FpuValue {
    Raw80(Raw80),
    F64(f64),
    F32(f32),
}

/// x87 binary NaN propagation: prefer a quiet NaN over a signaling one,
/// then the larger significand, preferring positive on an otherwise equal pair.
pub(super) fn propagate_nan(xr: Raw80, yr: Raw80) -> Option<(FpuValue, u16)> {
    if !xr.is_nan() && !yr.is_nan() {
        return None;
    }
    let x_quiet = xr.is_nan() && !xr.is_signaling_nan();
    let y_quiet = yr.is_nan() && !yr.is_signaling_nan();
    let chosen = if x_quiet != y_quiet {
        if x_quiet { xr } else { yr }
    } else if !yr.is_nan() {
        xr
    } else if !xr.is_nan() {
        yr
    } else if xr.significand > yr.significand
        || (xr.significand == yr.significand && xr.sign_exponent < yr.sign_exponent)
    {
        xr
    } else {
        yr
    };
    Some((
        FpuValue::Raw80(chosen.quiet()),
        if yr.is_signaling_nan() || xr.is_signaling_nan() {
            INVALID
        } else {
            0
        },
    ))
}

fn round_right(value: u64, shift: u32, negative: bool, rc: u16) -> (u64, bool) {
    if shift == 0 {
        return (value, false);
    }
    let truncated = if shift < 64 { value >> shift } else { 0 };
    let remainder = if shift < 64 {
        value & (u64::MAX >> (64 - shift))
    } else {
        value
    };
    let up = match rc {
        0 if shift <= 64 => {
            let half = 1u64 << (shift - 1);
            remainder > half || (remainder == half && truncated & 1 != 0)
        }
        1 => negative && remainder != 0,
        2 => !negative && remainder != 0,
        _ => false,
    };
    (truncated + u64::from(up), remainder != 0)
}

impl Raw80 {
    pub fn from_i64(value: i64) -> Self {
        Self::from_magnitude(value.unsigned_abs(), value < 0)
    }

    fn from_magnitude(value: u64, negative: bool) -> Self {
        let sign = if negative { 0x8000 } else { 0 };
        if value == 0 {
            return Self {
                significand: 0,
                sign_exponent: sign,
            };
        }
        let shift = value.leading_zeros();
        Self {
            significand: value << shift,
            sign_exponent: sign | (16383 + 63 - shift as u16),
        }
    }

    fn from_ieee(bits: u64, fraction: u32, exponent_bits: u32) -> Self {
        let sign = ((bits >> (fraction + exponent_bits)) as u16) << 15;
        let max_exp = (1u64 << exponent_bits) - 1;
        let exponent = (bits >> fraction) & max_exp;
        let mantissa = bits & ((1u64 << fraction) - 1);
        let bias = (1i32 << (exponent_bits - 1)) - 1;
        if exponent == max_exp {
            return Self {
                significand: (1u64 << 63) | (mantissa << (63 - fraction)),
                sign_exponent: sign | 0x7fff,
            };
        }
        if exponent == 0 {
            if mantissa == 0 {
                return Self {
                    significand: 0,
                    sign_exponent: sign,
                };
            }
            let shift = mantissa.leading_zeros();
            return Self {
                significand: mantissa << shift,
                sign_exponent: sign
                    | (16383 + 1 - bias - fraction as i32 + 63 - shift as i32) as u16,
            };
        }
        Self {
            significand: ((1u64 << fraction) | mantissa) << (63 - fraction),
            sign_exponent: sign | (16383 + exponent as i32 - bias) as u16,
        }
    }

    pub fn from_f64(value: f64) -> Self {
        Self::from_ieee(value.to_bits(), 52, 11)
    }
    pub fn from_f32(value: f32) -> Self {
        Self::from_ieee(value.to_bits() as u64, 23, 8)
    }
    pub fn is_negative(self) -> bool {
        self.sign_exponent & 0x8000 != 0
    }
    pub fn exponent(self) -> u16 {
        self.sign_exponent & 0x7fff
    }
    pub fn is_zero(self) -> bool {
        self.exponent() == 0 && self.significand == 0
    }
    pub fn is_nan(self) -> bool {
        self.exponent() == 0x7fff && self.significand != 1 << 63
    }
    pub fn is_signaling_nan(self) -> bool {
        self.is_nan() && self.significand & (1 << 62) == 0
    }
    pub fn is_infinite(self) -> bool {
        self.exponent() == 0x7fff && self.significand == 1 << 63
    }
    pub fn is_denormal(self) -> bool {
        self.exponent() == 0 && self.significand != 0
    }
    pub fn is_supported(self) -> bool {
        // Pseudo-denormals and unnormal/unsupported encodings are deferred.
        (self.exponent() == 0) == (self.significand & (1 << 63) == 0)
    }
    pub fn quiet(mut self) -> Self {
        self.significand |= 1 << 62;
        self
    }
    pub fn with_sign(mut self, negative: bool) -> Self {
        self.sign_exponent = self.exponent() | if negative { 0x8000 } else { 0 };
        self
    }

    /// Convert without narrowing through f64 first. RC is the x87 two-bit field.
    pub(crate) fn to_ieee(self, fraction: u32, exponent_bits: u32, rc: u16) -> (u64, u16) {
        let sign = u64::from(self.is_negative()) << (fraction + exponent_bits);
        let max_exp = (1u64 << exponent_bits) - 1;
        let inf = max_exp << fraction;
        if self.is_nan() {
            let payload = (self.significand >> (63 - fraction)) & ((1 << fraction) - 1);
            return (
                sign | inf | payload | (1 << (fraction - 1)),
                if self.is_signaling_nan() { INVALID } else { 0 },
            );
        }
        if self.is_infinite() {
            return (sign | inf, 0);
        }
        if self.is_zero() {
            return (sign, 0);
        }
        let bias = (1i32 << (exponent_bits - 1)) - 1;
        let shift = self.significand.leading_zeros();
        let sig = self.significand << shift;
        let exponent = i32::from(self.exponent().max(1)) - 16383 - shift as i32;
        let min_exponent = 1 - bias;
        let extra = (min_exponent - exponent).max(0) as u32;
        let (mut rounded, inexact) =
            round_right(sig, 63 - fraction + extra, self.is_negative(), rc);
        let mut result_exponent = exponent.max(min_exponent);
        if rounded >= 1 << (fraction + 1) {
            rounded >>= 1;
            result_exponent += 1;
        }
        if result_exponent > bias {
            let to_inf =
                rc == 0 || (rc == 1 && self.is_negative()) || (rc == 2 && !self.is_negative());
            return (
                sign | if to_inf { inf } else { inf - 1 },
                OVERFLOW | PRECISION,
            );
        }
        let mut flags = if inexact { PRECISION } else { 0 };
        if rounded < 1 << fraction {
            if inexact {
                flags |= UNDERFLOW;
            }
            (sign | rounded, flags)
        } else {
            (
                sign | (((result_exponent + bias) as u64) << fraction)
                    | (rounded & ((1 << fraction) - 1)),
                flags,
            )
        }
    }

    pub(crate) fn to_f64(self) -> (f64, u16) {
        let (bits, flags) = self.to_ieee(52, 11, 0);
        (f64::from_bits(bits), flags)
    }

    pub(crate) fn to_integer(self, width: u32, rc: u16) -> (u64, u16) {
        let indefinite = 1u64 << (width - 1);
        if self.is_nan() || self.is_infinite() || !self.is_supported() {
            return (indefinite, INVALID);
        }
        if self.is_zero() {
            return (0, 0);
        }
        let exponent = i32::from(self.exponent().max(1)) - 16383;
        if exponent > 63 {
            return (indefinite, INVALID);
        }
        let (magnitude, inexact) = round_right(
            self.significand,
            (63 - exponent) as u32,
            self.is_negative(),
            rc,
        );
        let signed = if self.is_negative() {
            -(magnitude as i128)
        } else {
            magnitude as i128
        };
        let limit = 1i128 << (width - 1);
        if signed < -limit || signed >= limit {
            return (indefinite, INVALID);
        }
        (signed as u64, if inexact { PRECISION } else { 0 })
    }

    pub(crate) fn round_integer(self, rc: u16) -> (Self, u16) {
        if self.is_nan() {
            return (
                self.quiet(),
                if self.is_signaling_nan() { INVALID } else { 0 },
            );
        }
        let exponent = i32::from(self.exponent().max(1)) - 16383;
        if self.is_zero() || self.is_infinite() || exponent >= 63 {
            return (self, 0);
        }
        let (value, inexact) = round_right(
            self.significand,
            (63 - exponent) as u32,
            self.is_negative(),
            rc,
        );
        (
            Self::from_magnitude(value, self.is_negative()),
            if inexact { PRECISION } else { 0 },
        )
    }

    pub(crate) fn compare(self, other: Self) -> core::cmp::Ordering {
        use core::cmp::Ordering;
        if self.is_zero() && other.is_zero() {
            return Ordering::Equal;
        }
        if self.is_negative() != other.is_negative() {
            return if self.is_negative() {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let order = (self.exponent(), self.significand).cmp(&(other.exponent(), other.significand));
        if self.is_negative() {
            order.reverse()
        } else {
            order
        }
    }
}

impl FpuValue {
    pub(crate) fn tag(self) -> u16 {
        match self {
            Self::Raw80(raw) => {
                if raw.is_zero() {
                    1
                } else if raw.is_nan()
                    || raw.is_infinite()
                    || raw.is_denormal()
                    || !raw.is_supported()
                {
                    2
                } else {
                    0
                }
            }
            Self::F64(v) => {
                if v == 0.0 {
                    1
                } else if !v.is_finite() {
                    2
                } else {
                    0
                }
            }
            Self::F32(v) => {
                if v == 0.0 {
                    1
                } else if !v.is_finite() {
                    2
                } else {
                    0
                }
            }
        }
    }

    pub(crate) fn is_negative_nonzero(self) -> bool {
        match self {
            Self::Raw80(raw) => raw.is_negative() && !raw.is_zero(),
            Self::F64(v) => v < 0.0,
            Self::F32(v) => v < 0.0,
        }
    }
    pub(crate) fn is_nan(self) -> bool {
        match self {
            Self::Raw80(raw) => raw.is_nan(),
            Self::F64(v) => v.is_nan(),
            Self::F32(v) => v.is_nan(),
        }
    }
    pub(crate) fn is_signaling_nan(self) -> bool {
        match self {
            Self::Raw80(raw) => raw.is_signaling_nan(),
            Self::F64(v) => v.is_nan() && v.to_bits() & (1 << 51) == 0,
            Self::F32(v) => v.is_nan() && v.to_bits() & (1 << 22) == 0,
        }
    }
    pub(crate) fn is_denormal(self) -> bool {
        // FLD m32/m64 normalized the input into the architectural register.
        matches!(self, Self::Raw80(raw) if raw.is_denormal())
    }

    pub fn raw(self) -> Raw80 {
        match self {
            Self::Raw80(raw) => raw,
            Self::F64(value) => Raw80::from_f64(value),
            Self::F32(value) => Raw80::from_f32(value),
        }
    }
    pub(crate) fn approximate(self) -> (f64, u16) {
        match self {
            Self::F64(value) => (value, 0),
            Self::F32(value) => (value as f64, 0),
            Self::Raw80(raw) => raw.to_f64(),
        }
    }
    pub(crate) fn sign(self, absolute: bool) -> Self {
        match self {
            Self::Raw80(raw) => Self::Raw80(raw.with_sign(!absolute && !raw.is_negative())),
            Self::F64(v) => Self::F64(f64::from_bits(if absolute {
                v.to_bits() & !(1 << 63)
            } else {
                v.to_bits() ^ (1 << 63)
            })),
            Self::F32(v) => Self::F32(f32::from_bits(if absolute {
                v.to_bits() & !(1 << 31)
            } else {
                v.to_bits() ^ (1 << 31)
            })),
        }
    }
}
