//! Integer, logical, pack/unpack, shift and shuffle operations on MMX (8
//! bytes) and XMM (16 bytes) values held as little-endian byte slices.
//!
//! Each operation runs on fixed-length typed arrays (the `map!` family), so
//! the compiler can vectorize it with whatever SIMD the build target
//! guarantees (SSE2 on x86-64, simd128 on Wasm when enabled, NEON on AArch64)
//! without `core::arch` (docs/PERFORMANCE.md).
use core::mem::size_of;

#[inline]
pub(super) fn get(bytes: &[u8], lane: usize, width: usize) -> u64 {
    let at = lane * width;
    match width {
        1 => u64::from(bytes[at]),
        2 => u64::from(u16::from_le_bytes([bytes[at], bytes[at + 1]])),
        4 => u64::from(u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())),
        _ => u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()),
    }
}

#[inline]
pub(super) fn put(bytes: &mut [u8], lane: usize, width: usize, value: u64) {
    let at = lane * width;
    match width {
        1 => bytes[at] = value as u8,
        2 => bytes[at..at + 2].copy_from_slice(&(value as u16).to_le_bytes()),
        4 => bytes[at..at + 4].copy_from_slice(&(value as u32).to_le_bytes()),
        _ => bytes[at..at + 8].copy_from_slice(&value.to_le_bytes()),
    }
}

/// The first `N` lanes of type `T` of `bytes`.
macro_rules! load {
    ($bytes:expr, $t:ty, $n:expr) => {{
        let mut out = [0 as $t; $n];
        for (lane, chunk) in out.iter_mut().zip($bytes.chunks_exact(size_of::<$t>())) {
            *lane = <$t>::from_le_bytes(chunk.try_into().unwrap());
        }
        out
    }};
}

macro_rules! store {
    ($bytes:expr, $lanes:expr, $t:ty) => {
        for (chunk, lane) in $bytes.chunks_exact_mut(size_of::<$t>()).zip($lanes.iter()) {
            chunk.copy_from_slice(&lane.to_le_bytes());
        }
    };
}

/// `dst[i] = f(dst[i], src[i])` over the lanes of type `$t` of an 8- or
/// 16-byte register, with a constant lane count for each size.
macro_rules! map {
    ($dst:expr, $src:expr, $t:ty, |$a:ident, $b:ident| $e:expr) => {{
        if $dst.len() == 16 {
            map!(@n 16, $dst, $src, $t, |$a, $b| $e)
        } else {
            map!(@n 8, $dst, $src, $t, |$a, $b| $e)
        }
    }};
    (@n $len:literal, $dst:expr, $src:expr, $t:ty, |$a:ident, $b:ident| $e:expr) => {{
        const N: usize = $len / size_of::<$t>();
        let x = load!(&$dst[..$len], $t, N);
        let y = load!(&$src[..$len], $t, N);
        let mut r = [0 as $t; N];
        for i in 0..N {
            let ($a, $b) = (x[i], y[i]);
            r[i] = $e;
        }
        store!(&mut $dst[..$len], r, $t);
    }};
}

/// The two-operand integer operations that work lane by lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Lane {
    Add,
    Sub,
    AddSigned,
    AddUnsigned,
    SubSigned,
    SubUnsigned,
    MulLow,
    MulHigh,
    MulHighUnsigned,
    CompareEqual,
    CompareGreater,
    Average,
    MinUnsigned,
    MaxUnsigned,
    MinSigned,
    MaxSigned,
}

pub(super) fn lanes(dst: &mut [u8], src: &[u8], width: usize, op: Lane) {
    use Lane::*;
    match (op, width) {
        (Add, 1) => map!(dst, src, u8, |a, b| a.wrapping_add(b)),
        (Add, 2) => map!(dst, src, u16, |a, b| a.wrapping_add(b)),
        (Add, 4) => map!(dst, src, u32, |a, b| a.wrapping_add(b)),
        (Add, _) => map!(dst, src, u64, |a, b| a.wrapping_add(b)),
        (Sub, 1) => map!(dst, src, u8, |a, b| a.wrapping_sub(b)),
        (Sub, 2) => map!(dst, src, u16, |a, b| a.wrapping_sub(b)),
        (Sub, 4) => map!(dst, src, u32, |a, b| a.wrapping_sub(b)),
        (Sub, _) => map!(dst, src, u64, |a, b| a.wrapping_sub(b)),
        (AddSigned, 1) => map!(dst, src, i8, |a, b| a.saturating_add(b)),
        (AddSigned, _) => map!(dst, src, i16, |a, b| a.saturating_add(b)),
        (AddUnsigned, 1) => map!(dst, src, u8, |a, b| a.saturating_add(b)),
        (AddUnsigned, _) => map!(dst, src, u16, |a, b| a.saturating_add(b)),
        (SubSigned, 1) => map!(dst, src, i8, |a, b| a.saturating_sub(b)),
        (SubSigned, _) => map!(dst, src, i16, |a, b| a.saturating_sub(b)),
        (SubUnsigned, 1) => map!(dst, src, u8, |a, b| a.saturating_sub(b)),
        (SubUnsigned, _) => map!(dst, src, u16, |a, b| a.saturating_sub(b)),
        (MulLow, _) => map!(dst, src, i16, |a, b| a.wrapping_mul(b)),
        (MulHigh, _) => map!(dst, src, i16, |a, b| ((i32::from(a) * i32::from(b)) >> 16)
            as i16),
        (MulHighUnsigned, _) => {
            map!(dst, src, u16, |a, b| ((u32::from(a) * u32::from(b)) >> 16)
                as u16)
        }
        (CompareEqual, 1) => map!(dst, src, u8, |a, b| if a == b { u8::MAX } else { 0 }),
        (CompareEqual, 2) => map!(dst, src, u16, |a, b| if a == b { u16::MAX } else { 0 }),
        (CompareEqual, _) => map!(dst, src, u32, |a, b| if a == b { u32::MAX } else { 0 }),
        (CompareGreater, 1) => map!(dst, src, i8, |a, b| if a > b { -1 } else { 0 }),
        (CompareGreater, 2) => map!(dst, src, i16, |a, b| if a > b { -1 } else { 0 }),
        (CompareGreater, _) => map!(dst, src, i32, |a, b| if a > b { -1 } else { 0 }),
        (Average, 1) => map!(
            dst,
            src,
            u8,
            |a, b| ((u16::from(a) + u16::from(b) + 1) >> 1) as u8
        ),
        (Average, _) => {
            map!(
                dst,
                src,
                u16,
                |a, b| ((u32::from(a) + u32::from(b) + 1) >> 1) as u16
            )
        }
        (MinUnsigned, _) => map!(dst, src, u8, |a, b| a.min(b)),
        (MaxUnsigned, _) => map!(dst, src, u8, |a, b| a.max(b)),
        (MinSigned, _) => map!(dst, src, i16, |a, b| a.min(b)),
        (MaxSigned, _) => map!(dst, src, i16, |a, b| a.max(b)),
    }
}

/// Bitwise operations on whole registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Logic {
    And,
    AndNot,
    Or,
    Xor,
}

pub(super) fn logic(dst: &mut [u8], src: &[u8], op: Logic) {
    match op {
        Logic::And => map!(dst, src, u64, |a, b| a & b),
        Logic::AndNot => map!(dst, src, u64, |a, b| !a & b),
        Logic::Or => map!(dst, src, u64, |a, b| a | b),
        Logic::Xor => map!(dst, src, u64, |a, b| a ^ b),
    }
}

/// PMULUDQ: each 64-bit lane is the product of the low dwords.
pub(super) fn mul_unsigned_dword(dst: &mut [u8], src: &[u8]) {
    map!(dst, src, u64, |a, b| (a & 0xffff_ffff) * (b & 0xffff_ffff));
}

/// PMADDWD: each dword is the sum of two signed word products (wrapping:
/// two 0x8000 * 0x8000 products give 0x80000000).
pub(super) fn multiply_add(dst: &mut [u8], src: &[u8]) {
    let n = dst.len() / 2;
    let x = load!(dst, i16, 8);
    let y = load!(src, i16, 8);
    let mut r = [0i32; 4];
    for (i, lane) in r.iter_mut().enumerate().take(n / 2) {
        let p = |k: usize| i32::from(x[k]) * i32::from(y[k]);
        *lane = p(2 * i).wrapping_add(p(2 * i + 1));
    }
    store!(dst, r, i32);
}

/// PSADBW: each qword holds the sum of absolute byte differences.
pub(super) fn sum_abs_diff(dst: &mut [u8], src: &[u8]) {
    let mut r = [0u64; 2];
    for (lane, sum) in r.iter_mut().enumerate().take(dst.len() / 8) {
        *sum = (0..8)
            .map(|k| u64::from(dst[lane * 8 + k].abs_diff(src[lane * 8 + k])))
            .sum();
    }
    store!(dst, r, u64);
}

/// PACKSSWB/PACKSSDW (signed) and PACKUSWB (unsigned): `dst` lanes then
/// `src` lanes, each narrowed to half `width` with saturation.
pub(super) fn pack(dst: &mut [u8], src: &[u8], width: usize, unsigned: bool) {
    let len = dst.len();
    let mut out = [0u8; 16];
    if width == 2 {
        let x = load!(dst, i16, 8);
        let y = load!(src[..len], i16, 8);
        let n = len / 2;
        for i in 0..2 * n {
            let v = if i < n { x[i] } else { y[i - n] };
            out[i] = if unsigned {
                v.clamp(0, 255) as u8
            } else {
                v.clamp(-128, 127) as i8 as u8
            };
        }
    } else {
        let x = load!(dst, i32, 4);
        let y = load!(src[..len], i32, 4);
        let n = len / 4;
        let mut r = [0i16; 8];
        for i in 0..2 * n {
            let v = if i < n { x[i] } else { y[i - n] };
            r[i] = v.clamp(-32768, 32767) as i16;
        }
        store!(out, r, i16);
    }
    dst.copy_from_slice(&out[..len]);
}

/// PUNPCKL*/PUNPCKH* and UNPCKL/H PS/PD: interleave the low (or high)
/// halves, `dst` lane first.
pub(super) fn unpack(dst: &mut [u8], src: &[u8], width: usize, high: bool) {
    let len = dst.len();
    let half = len / 2;
    let base = if high { half } else { 0 };
    let mut out = [0u8; 16];
    for i in 0..half / width {
        let at = base + i * width;
        out[2 * i * width..][..width].copy_from_slice(&dst[at..at + width]);
        out[(2 * i + 1) * width..][..width].copy_from_slice(&src[at..at + width]);
    }
    dst.copy_from_slice(&out[..len]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Shift {
    Left,
    Right,
    Arithmetic,
}

/// PSLL/PSRL/PSRA by `count`; a count beyond the lane clears it (or fills
/// it with the sign).
pub(super) fn shift(dst: &mut [u8], width: usize, kind: Shift, count: u64) {
    macro_rules! shift_lanes {
        ($u:ty, $i:ty) => {{
            let bits = (size_of::<$u>() * 8) as u64;
            let zero = [0u8; 16];
            match kind {
                Shift::Left if count < bits => map!(dst, zero, $u, |a, _b| a << count),
                Shift::Right if count < bits => map!(dst, zero, $u, |a, _b| a >> count),
                Shift::Left | Shift::Right => dst.fill(0),
                Shift::Arithmetic => {
                    let count = count.min(bits - 1);
                    map!(dst, zero, $i, |a, _b| a >> count)
                }
            }
        }};
    }
    match width {
        2 => shift_lanes!(u16, i16),
        4 => shift_lanes!(u32, i32),
        _ => shift_lanes!(u64, i64),
    }
}

/// PSLLDQ/PSRLDQ: whole-register byte shift.
pub(super) fn shift_bytes(dst: &mut [u8], left: bool, count: u8) {
    let n = dst.len();
    let count = usize::from(count).min(n);
    let mut out = [0u8; 16];
    if left {
        out[count..n].copy_from_slice(&dst[..n - count]);
    } else {
        out[..n - count].copy_from_slice(&dst[count..]);
    }
    dst.copy_from_slice(&out[..n]);
}

/// PSHUFW/PSHUFD and the halves of PSHUFHW/PSHUFLW: lane i of `dst` (from
/// `offset`, four lanes) is lane imm[2i+1:2i] of `src` (from `offset`).
pub(super) fn shuffle(dst: &mut [u8], src: &[u8], width: usize, offset: usize, imm: u8) {
    for i in 0..4 {
        let pick = usize::from(imm >> (2 * i)) & 3;
        put(dst, offset + i, width, get(src, offset + pick, width));
    }
}

/// PMOVMSKB/MOVMSKPS/MOVMSKPD: the sign bit of each lane.
pub(super) fn sign_mask(src: &[u8], width: usize) -> u32 {
    (0..src.len() / width).fold(0, |mask, lane| {
        mask | (u32::from(src[lane * width + width - 1] >> 7) << lane)
    })
}
