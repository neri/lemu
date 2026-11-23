//! SSE/SSE2 floating point on raw bits.
//!
//! The default path uses the host's f32/f64 when MXCSR selects nearest
//! rounding with FTZ and DAZ off, every input is normal or zero, and the
//! result is normal and above the underflow boundary in magnitude (or an
//! exact zero). Its exception flags are exact:
//! IE/DE/ZE/OE/UE cannot occur there, and PE comes from an exactness check.
//! Everything else takes the cold path, a software implementation that
//! rounds in all four modes, applies DAZ/FTZ and sets every flag exactly.
//! NaN results never come from the host, so they do not depend on it (Wasm).
//!
//! Only masked responses exist: HarvardUME and FlatUME reject unmasking.

/// MXCSR flag bits.
pub(crate) const IE: u32 = 0x01;
pub(crate) const DE: u32 = 0x02;
pub(crate) const ZE: u32 = 0x04;
pub(crate) const OE: u32 = 0x08;
pub(crate) const UE: u32 = 0x10;
pub(crate) const PE: u32 = 0x20;
const DAZ: u32 = 1 << 6;
const FTZ: u32 = 1 << 15;

/// An IEEE binary format, values carried in the low bits of a u64.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Format {
    frac: u32,
    exp: u32,
}
pub(crate) const F32: Format = Format { frac: 23, exp: 8 };
pub(crate) const F64: Format = Format { frac: 52, exp: 11 };

impl Format {
    pub(crate) const fn bytes(self) -> usize {
        if self.frac == 23 { 4 } else { 8 }
    }
    const fn bias(self) -> i32 {
        (1 << (self.exp - 1)) - 1
    }
    const fn max_exp(self) -> u64 {
        (1 << self.exp) - 1
    }
    const fn sign(self) -> u64 {
        1 << (self.frac + self.exp)
    }
    const fn frac_mask(self) -> u64 {
        (1 << self.frac) - 1
    }
    const fn quiet(self) -> u64 {
        1 << (self.frac - 1)
    }
    const fn exponent(self, bits: u64) -> u64 {
        (bits >> self.frac) & self.max_exp()
    }
    /// The default QNaN ("real indefinite"): 0xFFC00000 / 0xFFF8000000000000.
    pub(crate) const fn indefinite(self) -> u64 {
        self.sign() | (self.max_exp() << self.frac) | self.quiet()
    }
    pub(crate) const fn is_nan(self, bits: u64) -> bool {
        self.exponent(bits) == self.max_exp() && bits & self.frac_mask() != 0
    }
    const fn is_snan(self, bits: u64) -> bool {
        self.is_nan(bits) && bits & self.quiet() == 0
    }
    const fn is_inf(self, bits: u64) -> bool {
        self.exponent(bits) == self.max_exp() && bits & self.frac_mask() == 0
    }
    const fn is_zero(self, bits: u64) -> bool {
        bits & !self.sign() == 0
    }
    const fn is_denormal(self, bits: u64) -> bool {
        self.exponent(bits) == 0 && bits & self.frac_mask() != 0
    }
    const fn is_normal_or_zero(self, bits: u64) -> bool {
        let e = self.exponent(bits);
        (e != 0 && e != self.max_exp()) || self.is_zero(bits)
    }
    const fn is_negative(self, bits: u64) -> bool {
        bits & self.sign() != 0
    }
    const fn zero(self, negative: bool) -> u64 {
        if negative { self.sign() } else { 0 }
    }
    const fn inf(self, negative: bool) -> u64 {
        self.zero(negative) | (self.max_exp() << self.frac)
    }
    const fn max_finite(self, negative: bool) -> u64 {
        self.inf(negative) - 1
    }
    /// Finite nonzero `bits` as `(negative, m, e)` with value m * 2^e.
    fn unpack(self, bits: u64) -> (bool, u128, i32) {
        let exp = self.exponent(bits);
        let frac = bits & self.frac_mask();
        let (m, e) = if exp == 0 {
            (frac, 1 - self.bias() - self.frac as i32)
        } else {
            (
                frac | (1 << self.frac),
                exp as i32 - self.bias() - self.frac as i32,
            )
        };
        (self.is_negative(bits), u128::from(m), e)
    }
    fn to_f64(self, bits: u64) -> f64 {
        if self == F32 {
            f64::from(f32::from_bits(bits as u32))
        } else {
            f64::from_bits(bits)
        }
    }
}

/// Rounding and flag state of one instruction: MXCSR in, flags out.
pub(crate) struct Env {
    mxcsr: u32,
    pub(crate) flags: u32,
}

impl Env {
    pub(crate) const fn new(mxcsr: u32) -> Self {
        Self { mxcsr, flags: 0 }
    }
    /// 0 nearest, 1 down, 2 up, 3 toward zero.
    const fn rc(&self) -> u32 {
        (self.mxcsr >> 13) & 3
    }
    const fn fast(&self) -> bool {
        self.mxcsr & (0x6000 | FTZ | DAZ) == 0
    }
    /// A source operand after DAZ, with DE for a denormal when DAZ is off.
    fn operand(&mut self, f: Format, bits: u64) -> u64 {
        if f.is_denormal(bits) {
            if self.mxcsr & DAZ != 0 {
                return f.zero(f.is_negative(bits));
            }
            self.flags |= DE;
        }
        bits
    }
    /// A source operand after DAZ, and whether it is a denormal that DE
    /// would report. Arithmetic reports DE only when neither IE nor ZE
    /// (higher-priority pre-computation exceptions) occurs.
    fn operand_deferred(&self, f: Format, bits: u64) -> (u64, bool) {
        if f.is_denormal(bits) {
            if self.mxcsr & DAZ != 0 {
                return (f.zero(f.is_negative(bits)), false);
            }
            return (bits, true);
        }
        (bits, false)
    }

    /// DAZ without DE (conversions to integer report no DE).
    fn daz(&self, f: Format, bits: u64) -> u64 {
        if self.mxcsr & DAZ != 0 && f.is_denormal(bits) {
            f.zero(f.is_negative(bits))
        } else {
            bits
        }
    }
}

/// `m` with `shift` low bits dropped, rounded by `rc` for a value of sign
/// `negative`; `sticky` stands for nonzero bits below `m`. Returns the
/// rounded integer and whether anything was dropped.
fn round_bits(m: u128, sticky: bool, shift: u32, negative: bool, rc: u32) -> (u128, bool) {
    if shift == 0 {
        return (m, sticky);
    }
    let (q, drop, half) = if shift < 128 {
        (m >> shift, m & ((1 << shift) - 1), 1u128 << (shift - 1))
    } else if shift == 128 {
        (0, m, 1 << 127)
    } else {
        (0, m, u128::MAX) // below half whatever m is
    };
    let inexact = drop != 0 || sticky;
    let up = match rc {
        0 => drop > half || (drop == half && (sticky || q & 1 == 1)),
        1 => negative && inexact,
        2 => !negative && inexact,
        _ => false,
    };
    (q + u128::from(up), inexact)
}

/// Rounds the nonzero value (m + sticky) * 2^e to `f` with the masked
/// responses: overflow (OE, PE), underflow with tininess after rounding (UE
/// when inexact, or always with FTZ, which returns zero), inexact (PE).
fn round_pack(f: Format, env: &mut Env, negative: bool, m: u128, e: i32, sticky: bool) -> u64 {
    debug_assert!(m != 0);
    let lz = m.leading_zeros();
    let m = m << lz;
    let e = e - lz as i32;
    let top = e + 127; // exponent of the leading bit
    let p = f.frac + 1;
    let rc = env.rc();
    let emin = 1 - f.bias();
    // Tininess: the result rounded with an unbounded exponent is below 2^emin.
    let (unbounded, _) = round_bits(m, sticky, 128 - p, negative, rc);
    let tiny = top + i32::from(unbounded >> p != 0) < emin;
    if tiny && env.mxcsr & FTZ != 0 {
        env.flags |= UE | PE;
        return f.zero(negative);
    }
    let shift = if top >= emin {
        128 - p
    } else {
        (128 - p as i32 + (emin - top)).min(200) as u32
    };
    let (mut q, inexact) = round_bits(m, sticky, shift, negative, rc);
    let mut biased = if top >= emin { top + f.bias() } else { 0 };
    if q >> p != 0 {
        q >>= 1;
        biased += 1;
    } else if biased == 0 && q >> f.frac != 0 {
        biased = 1; // rounded up into the smallest normal
    }
    if biased >= f.max_exp() as i32 {
        env.flags |= OE | PE;
        let to_inf = match rc {
            0 => true,
            1 => negative,
            2 => !negative,
            _ => false,
        };
        return if to_inf {
            f.inf(negative)
        } else {
            f.max_finite(negative)
        };
    }
    if inexact {
        env.flags |= PE;
        if tiny {
            env.flags |= UE;
        }
    }
    f.zero(negative) | ((biased as u64) << f.frac) | (q as u64 & f.frac_mask())
}

/// x86 NaN result of a two-source operation: the first source if it is a
/// NaN, otherwise the second, quieted; IE for any SNaN.
fn nan2(f: Format, env: &mut Env, a: u64, b: u64) -> u64 {
    if f.is_snan(a) || f.is_snan(b) {
        env.flags |= IE;
    }
    if f.is_nan(a) {
        a | f.quiet()
    } else {
        b | f.quiet()
    }
}

fn invalid(f: Format, env: &mut Env) -> u64 {
    env.flags |= IE;
    f.indefinite()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    /// sqrt(b); `a` is ignored.
    Sqrt,
}

/// `a op b` (or sqrt(b)) in format `f`: one lane of [`arith_lanes`].
/// `op` on lanes 0..`n` of `dst` and `src` (4 or 1 f32 lanes, 2 or 1 f64
/// lanes); the other lanes of `dst` are left alone.
pub(crate) fn arith_lanes(
    f: Format,
    env: &mut Env,
    op: Arith,
    dst: &mut [u8; 16],
    src: &[u8; 16],
    n: usize,
) {
    macro_rules! run {
        ($lanes:ident, $bits:ty, $n:literal) => {{
            const W: usize = core::mem::size_of::<$bits>();
            let load = |bytes: &[u8; 16]| -> [$bits; $n] {
                core::array::from_fn(|i| {
                    <$bits>::from_le_bytes(bytes[i * W..i * W + W].try_into().unwrap())
                })
            };
            let r = $lanes::<$n>(env, op, load(dst), load(src));
            for (i, r) in r.iter().enumerate() {
                dst[i * W..i * W + W].copy_from_slice(&r.to_le_bytes());
            }
        }};
    }
    match (f == F32, n) {
        (true, 4) => run!(lanes_f32, u32, 4),
        (true, _) => run!(lanes_f32, u32, 1),
        (false, 2) => run!(lanes_f64, u64, 2),
        (false, _) => run!(lanes_f64, u64, 1),
    }
}

/// `N` lanes of one operation. The default path uses the host's f32/f64 when
/// MXCSR selects nearest rounding with FTZ and DAZ off, both inputs are
/// normal or zero, and the result is normal and above the underflow boundary
/// in magnitude (or an exact zero). When every lane qualifies, all lanes are
/// computed in one vectorizable loop per operation; otherwise the lanes that
/// do not qualify take the cold path. Lanes stay raw bits, so NaNs never pass
/// through host float registers.
macro_rules! host_lanes {
    ($name:ident, $t:ty, $bits:ty, $f:expr, $sqrt:path, $exact:expr) => {
        fn $name<const N: usize>(
            env: &mut Env,
            op: Arith,
            a: [$bits; N],
            b: [$bits; N],
        ) -> [$bits; N] {
            /// All `N` lanes on the host, or `None` (nothing changed) when
            /// a lane does not qualify.
            #[inline(always)]
            fn host<const N: usize>(
                env: &mut Env,
                op: Arith,
                a: [$bits; N],
                b: [$bits; N],
            ) -> Option<[$bits; N]> {
                let inputs = (0..N).all(|i| {
                    let b = u64::from(b[i]);
                    $f.is_normal_or_zero(b)
                        && if op == Arith::Sqrt {
                            !$f.is_negative(b) || $f.is_zero(b)
                        } else {
                            $f.is_normal_or_zero(u64::from(a[i]))
                        }
                });
                if !inputs {
                    return None;
                }
                let x: [$t; N] = a.map(<$t>::from_bits);
                let y: [$t; N] = b.map(<$t>::from_bits);
                let mut r = [0 as $t; N];
                match op {
                    Arith::Add => (0..N).for_each(|i| r[i] = x[i] + y[i]),
                    Arith::Sub => (0..N).for_each(|i| r[i] = x[i] - y[i]),
                    Arith::Mul => (0..N).for_each(|i| r[i] = x[i] * y[i]),
                    Arith::Div => (0..N).for_each(|i| r[i] = x[i] / y[i]),
                    Arith::Sqrt => (0..N).for_each(|i| r[i] = $sqrt(y[i])),
                }
                // A tiny exact value can round to MIN_POSITIVE and still raise UE.
                // The square root of a positive normal is normal and above
                // MIN_POSITIVE; either signed zero has an exact root.
                let results = op == Arith::Sqrt
                    || (0..N).all(|i| {
                        (r[i].is_normal() && r[i].abs() != <$t>::MIN_POSITIVE)
                            || (r[i] == 0.0
                                && match op {
                                    // A zero sum of normal inputs is exact (Sterbenz).
                                    Arith::Add | Arith::Sub | Arith::Sqrt => true,
                                    Arith::Mul => x[i] == 0.0 || y[i] == 0.0,
                                    Arith::Div => x[i] == 0.0 && y[i] != 0.0,
                                })
                    });
                if !results {
                    return None;
                }
                // PE is sticky. With masked exceptions, detecting it again
                // cannot change the architectural state or this result.
                let precision_known = env.mxcsr & super::MXCSR_EXCEPTION_MASKS
                    == super::MXCSR_EXCEPTION_MASKS
                    && (env.mxcsr | env.flags) & PE != 0;
                let inexact = !precision_known
                    && match op {
                        // TwoSum: the rounding error of r = x + y, exactly.
                        Arith::Add | Arith::Sub => (0..N).any(|i| {
                            let y = if op == Arith::Sub { -y[i] } else { y[i] };
                            let yy = r[i] - x[i];
                            (x[i] - (r[i] - yy)) + (y - yy) != 0.0
                        }),
                        _ => {
                            let exact: fn(Arith, $t, $t, $t) -> bool = $exact;
                            (0..N).any(|i| !exact(op, x[i], y[i], r[i]))
                        }
                    };
                if inexact {
                    env.flags |= PE;
                }
                Some(r.map(<$t>::to_bits))
            }

            /// Lane by lane: each lane on the host when it qualifies, otherwise
            /// in software.
            #[cold]
            #[inline(never)]
            fn mixed<const N: usize>(
                env: &mut Env,
                op: Arith,
                a: [$bits; N],
                b: [$bits; N],
            ) -> [$bits; N] {
                let fast = env.fast();
                core::array::from_fn(|i| {
                    match fast.then(|| host::<1>(env, op, [a[i]], [b[i]])).flatten() {
                        Some([r]) => r,
                        None => cold($f, env, op, u64::from(a[i]), u64::from(b[i])) as $bits,
                    }
                })
            }

            if env.fast() {
                if let Some(r) = host::<N>(env, op, a, b) {
                    return r;
                }
            }
            mixed::<N>(env, op, a, b)
        }
    };
}

// f32: products of two singles are exact in f64. libm (no_std): sqrt/sqrtf
// are correctly rounded.
host_lanes!(lanes_f32, f32, u32, F32, libm::sqrtf, |op, a, b, r| {
    let (a, b, r) = (f64::from(a), f64::from(b), f64::from(r));
    match op {
        Arith::Mul => a * b == r,
        Arith::Div => r * b == a,
        _ => r * r == b,
    }
});
// f64: the integer check of `exact_product`.
host_lanes!(lanes_f64, f64, u64, F64, libm::sqrt, |op, a, b, r| {
    let (a, b, r) = (a.to_bits(), b.to_bits(), r.to_bits());
    match op {
        Arith::Mul => exact_product(F64, a, b, r),
        Arith::Div => exact_product(F64, r, b, a),
        _ => exact_product(F64, r, r, b),
    }
});

/// Whether x * y equals z exactly (x, y, z finite; zero handled).
fn exact_product(f: Format, x: u64, y: u64, z: u64) -> bool {
    if f.is_zero(x) || f.is_zero(y) || f.is_zero(z) {
        return (f.is_zero(x) || f.is_zero(y)) == f.is_zero(z);
    }
    let (_, mx, ex) = f.unpack(x);
    let (_, my, ey) = f.unpack(y);
    let (_, mz, ez) = f.unpack(z);
    let (mut mp, mut ep) = (mx * my, ex + ey);
    let tz = mp.trailing_zeros();
    mp >>= tz;
    ep += tz as i32;
    let tz = mz.trailing_zeros();
    mp == mz >> tz && ep == ez + tz as i32
}

fn cold(f: Format, env: &mut Env, op: Arith, a: u64, b: u64) -> u64 {
    if op == Arith::Sqrt {
        if f.is_nan(b) {
            return nan2(f, env, b, b);
        }
        let (b, denormal) = env.operand_deferred(f, b);
        if f.is_zero(b) {
            return b;
        }
        if f.is_negative(b) {
            return invalid(f, env);
        }
        if denormal {
            env.flags |= DE;
        }
        if f.is_inf(b) {
            return b;
        }
        let (_, m, e) = f.unpack(b);
        // Even exponent, then the radicand scaled to about 2^126.
        let (m, e) = if e & 1 != 0 { (m << 1, e - 1) } else { (m, e) };
        let shift = (126 - (127 - m.leading_zeros() as i32)) & !1;
        let radicand = m << shift;
        let root = radicand.isqrt();
        let sticky = root * root != radicand;
        return round_pack(f, env, false, root, (e - shift) / 2, sticky);
    }
    if f.is_nan(a) || f.is_nan(b) {
        return nan2(f, env, a, b);
    }
    let (a, da) = env.operand_deferred(f, a);
    let (b, db) = env.operand_deferred(f, b);
    let b = if op == Arith::Sub { b ^ f.sign() } else { b };
    let (na, nb) = (f.is_negative(a), f.is_negative(b));
    if !da && !db {
        return cold_finite(f, env, op, a, b, na, nb);
    }
    // IE and ZE suppress DE for this lane, regardless of earlier lanes.
    let mut lane = Env::new(env.mxcsr);
    let result = cold_finite(f, &mut lane, op, a, b, na, nb);
    if lane.flags & (IE | ZE) == 0 {
        lane.flags |= DE;
    }
    env.flags |= lane.flags;
    result
}

fn cold_finite(f: Format, env: &mut Env, op: Arith, a: u64, b: u64, na: bool, nb: bool) -> u64 {
    let rc = env.rc();
    match op {
        Arith::Add | Arith::Sub => {
            match (f.is_inf(a), f.is_inf(b)) {
                (true, true) if na != nb => return invalid(f, env),
                (true, _) => return a,
                (_, true) => return b,
                _ => {}
            }
            match (f.is_zero(a), f.is_zero(b)) {
                (true, true) => {
                    return f.zero(if na == nb { na } else { rc == 1 });
                }
                (true, false) => return b,
                (false, true) => return a,
                _ => {}
            }
            let (_, ma, ea) = f.unpack(a);
            let (_, mb, eb) = f.unpack(b);
            // Align on the larger exponent, 64 guard bits; far bits are sticky.
            let ((mx, ex, nx), (my, ey, ny)) = if ea >= eb {
                ((ma, ea, na), (mb, eb, nb))
            } else {
                ((mb, eb, nb), (ma, ea, na))
            };
            let big = mx << 64;
            let distance = (ex - ey) as u32;
            let (small, sticky) = if distance <= 64 {
                (my << (64 - distance), false)
            } else if distance - 64 < 128 {
                let s = distance - 64;
                (my >> s, my & ((1 << s) - 1) != 0)
            } else {
                (0, true)
            };
            let e = ex - 64;
            if nx == ny {
                return round_pack(f, env, nx, big + small, e, sticky);
            }
            // big > small (or equal with no sticky) unless the exponents tie.
            let (diff, negative) = if big >= small {
                (big - small, nx)
            } else {
                (small - big, ny)
            };
            if diff == 0 && !sticky {
                return f.zero(rc == 1);
            }
            // A sticky remainder belongs to the subtracted operand.
            let (diff, sticky) = if sticky {
                (diff - 1, true)
            } else {
                (diff, false)
            };
            if diff == 0 {
                return round_pack(f, env, negative, 1, e - 1, true);
            }
            round_pack(f, env, negative, diff, e, sticky)
        }
        Arith::Mul => {
            let negative = na != nb;
            if f.is_inf(a) || f.is_inf(b) {
                if f.is_zero(a) || f.is_zero(b) {
                    return invalid(f, env);
                }
                return f.inf(negative);
            }
            if f.is_zero(a) || f.is_zero(b) {
                return f.zero(negative);
            }
            let (_, ma, ea) = f.unpack(a);
            let (_, mb, eb) = f.unpack(b);
            round_pack(f, env, negative, ma * mb, ea + eb, false)
        }
        Arith::Div => {
            let negative = na != nb;
            match (f.is_inf(a), f.is_inf(b), f.is_zero(a), f.is_zero(b)) {
                (true, true, ..) | (_, _, true, true) => return invalid(f, env),
                (true, ..) => return f.inf(negative),
                (_, true, ..) => return f.zero(negative),
                (_, _, true, _) => return f.zero(negative),
                (_, _, _, true) => {
                    env.flags |= ZE;
                    return f.inf(negative);
                }
                _ => {}
            }
            let (_, ma, ea) = f.unpack(a);
            let (_, mb, eb) = f.unpack(b);
            let sa = ma.leading_zeros();
            let sb = mb.leading_zeros() - 64;
            let (num, den) = (ma << sa, mb << sb);
            let q = num / den;
            let sticky = num % den != 0;
            round_pack(f, env, negative, q, ea - sa as i32 - eb + sb as i32, sticky)
        }
        Arith::Sqrt => unreachable!(),
    }
}

/// MINPS/MAXPS and the scalar forms: a NaN or a pair of zeros returns the
/// second source unchanged (IE for any NaN); otherwise the smaller/larger.
pub(crate) fn min_max(f: Format, env: &mut Env, max: bool, a: u64, b: u64) -> u64 {
    if f.is_nan(a) || f.is_nan(b) {
        env.flags |= IE;
        // DAZ applies to the returned operand too.
        return env.daz(f, b);
    }
    let a = env.operand(f, a);
    let b = env.operand(f, b);
    if f.is_zero(a) && f.is_zero(b) {
        return b;
    }
    let (x, y) = (f.to_f64(a), f.to_f64(b));
    let pick_a = if max { x > y } else { x < y };
    if pick_a { a } else { b }
}

/// CMPPS predicate `imm & 7` (EQ, LT, LE, UNORD, NEQ, NLT, NLE, ORD). LT,
/// LE, NLT and NLE signal IE on a QNaN too; the others only on an SNaN.
pub(crate) fn compare(f: Format, env: &mut Env, predicate: u8, a: u64, b: u64) -> bool {
    let predicate = predicate & 7;
    let signaling = matches!(predicate, 1 | 2 | 5 | 6);
    let unordered = f.is_nan(a) || f.is_nan(b);
    if unordered {
        if signaling || f.is_snan(a) || f.is_snan(b) {
            env.flags |= IE;
        }
    } else {
        // DE only for ordered inputs, as for arithmetic.
        let _ = (env.operand(f, a), env.operand(f, b));
    }
    let (x, y) = (f.to_f64(env.daz(f, a)), f.to_f64(env.daz(f, b)));
    match predicate {
        0 => !unordered && x == y,
        1 => !unordered && x < y,
        2 => !unordered && x <= y,
        3 => unordered,
        4 => unordered || x != y,
        5 => unordered || !(x < y),
        6 => unordered || !(x <= y),
        _ => !unordered,
    }
}

/// (U)COMISS/(U)COMISD: (ZF, PF, CF). COMIS signals IE on any NaN, UCOMIS
/// only on an SNaN.
pub(crate) fn compare_flags(
    f: Format,
    env: &mut Env,
    quiet: bool,
    a: u64,
    b: u64,
) -> (bool, bool, bool) {
    if f.is_nan(a) || f.is_nan(b) {
        if !quiet || f.is_snan(a) || f.is_snan(b) {
            env.flags |= IE;
        }
        return (true, true, true);
    }
    let a = env.operand(f, a);
    let b = env.operand(f, b);
    let (x, y) = (f.to_f64(a), f.to_f64(b));
    (x == y, false, x < y)
}

/// Format conversion. Widening is exact; narrowing rounds by MXCSR.RC.
/// A NaN keeps its sign and the upper payload bits and is quieted.
pub(crate) fn convert(from: Format, to: Format, env: &mut Env, bits: u64) -> u64 {
    if from.is_nan(bits) {
        if from.is_snan(bits) {
            env.flags |= IE;
        }
        let payload = bits & from.frac_mask();
        let payload = if to.frac >= from.frac {
            payload << (to.frac - from.frac)
        } else {
            payload >> (from.frac - to.frac)
        };
        return to.inf(from.is_negative(bits)) | payload | to.quiet();
    }
    let bits = env.operand(from, bits);
    // Widening a normal f32 is exact in every rounding mode. Keep NaN
    // payloads and denormal handling in the software paths.
    if from == F32 && to == F64 && from.is_normal_or_zero(bits) {
        return f64::from(f32::from_bits(bits as u32)).to_bits();
    }
    let negative = from.is_negative(bits);
    if from.is_inf(bits) {
        return to.inf(negative);
    }
    if from.is_zero(bits) {
        return to.zero(negative);
    }
    let (_, m, e) = from.unpack(bits);
    round_pack(to, env, negative, m, e, false)
}

/// Signed 32-bit integer to `to`, rounded by MXCSR.RC.
pub(crate) fn from_i32(to: Format, env: &mut Env, value: i32) -> u64 {
    if to == F64 {
        return f64::from(value).to_bits();
    }
    if value == 0 {
        return 0;
    }
    round_pack(
        to,
        env,
        value < 0,
        u128::from(value.unsigned_abs()),
        0,
        false,
    )
}

/// To a signed 32-bit integer, rounded by MXCSR.RC or truncated. A NaN or
/// a value out of range is IE with the integer indefinite 0x80000000; no DE.
pub(crate) fn to_i32(from: Format, env: &mut Env, bits: u64, truncate: bool) -> u32 {
    const INDEFINITE: u32 = 0x8000_0000;
    let bits = env.daz(from, bits);
    if from.is_nan(bits) || from.is_inf(bits) {
        env.flags |= IE;
        return INDEFINITE;
    }
    if from.is_zero(bits) {
        return 0;
    }
    let (negative, m, e) = from.unpack(bits);
    let rc = if truncate { 3 } else { env.rc() };
    let (magnitude, inexact) = if e >= 0 {
        if e > 40 {
            env.flags |= IE;
            return INDEFINITE;
        }
        (m << e, false)
    } else {
        round_bits(m, false, (-e) as u32, negative, rc)
    };
    let limit = if negative { 1u128 << 31 } else { (1 << 31) - 1 };
    if magnitude > limit {
        env.flags |= IE;
        return INDEFINITE;
    }
    if inexact {
        env.flags |= PE;
    }
    let value = magnitude as u32;
    if negative {
        value.wrapping_neg()
    } else {
        value
    }
}

/// RCPSS/RSQRTSS approximations. They ignore MXCSR: no flags, denormal
/// inputs act as zero and tiny results flush to zero. PLUM returns the
/// double-precision result rounded to single, which is within the
/// architectural 1.5 * 2^-12 relative error but not bit-identical to
/// Intel's tables.
pub(crate) fn reciprocal(bits: u32, sqrt: bool) -> u32 {
    let f = F32;
    let b = u64::from(bits);
    if f.is_nan(b) {
        return bits | f.quiet() as u32;
    }
    let negative = f.is_negative(b);
    if f.is_zero(b) || f.is_denormal(b) {
        return f.inf(negative) as u32;
    }
    if sqrt && negative {
        return f.indefinite() as u32;
    }
    let x = f64::from(f32::from_bits(bits));
    let r = if sqrt { 1.0 / libm::sqrt(x) } else { 1.0 / x };
    let r = r as f32;
    if r.is_subnormal() {
        return f.zero(negative) as u32;
    }
    r.to_bits()
}
