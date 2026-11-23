//! Results of the MMX/SSE/SSE2 forms. The engines check CR0/CR4, alignment
//! and the whole memory operand first, and call these last: a form either
//! completes or returns a fault with nothing changed. FXSAVE/FXRSTOR
//! ([`super::fxsave`]), CLFLUSH (an access check only) and
//! MASKMOVQ/MASKMOVDQU ([`mask_move`]) need the engine's memory access.
use ir86::prelude::{GprIndex32, SimdOp, SimdRegKind};

use super::float::{self, Arith, Env, F32, F64, Format};
use super::lanes::{self, Lane, Shift, get, put};
use super::{MXCSR_EXCEPTION_MASKS, MXCSR_MASK, mmx};
use crate::cpu::{ProcessorState, RtReg};
use crate::gpr::PartialRegister;

/// Why a shared SIMD step did not complete. Nothing has changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SimdFault {
    /// #GP(0): a reserved MXCSR bit.
    GeneralProtection,
    /// A value outside the implemented scope (an LDMXCSR or FXRSTOR that
    /// unmasks a numeric exception on HarvardUME or FlatUME).
    Unimplemented,
}

/// The ModR/M r/m operand: a register number, or the bytes read from memory
/// (`op.mem_size()` of them).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Rm<'a> {
    Reg(usize),
    Mem(&'a [u8]),
}

const fn starts_with(name: &[u8], prefix: &[u8]) -> bool {
    if name.len() < prefix.len() {
        return false;
    }
    let mut i = 0;
    while i < prefix.len() {
        if name[i] != prefix[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn is_numeric(op: SimdOp) -> bool {
    let m = op.mnemonic().as_bytes();
    let n = m.len();
    let float_suffix = n >= 2 && matches!(m[n - 2], b'P' | b'S') && matches!(m[n - 1], b'S' | b'D');
    let arithmetic = starts_with(m, b"ADD")
        || starts_with(m, b"SUB")
        || starts_with(m, b"MUL")
        || starts_with(m, b"DIV")
        || starts_with(m, b"SQRT")
        || starts_with(m, b"MIN")
        || starts_with(m, b"MAX")
        || starts_with(m, b"CMP");
    starts_with(m, b"CVT")
        || starts_with(m, b"COMIS")
        || starts_with(m, b"UCOMIS")
        || (arithmetic && float_suffix)
}

const NUMERIC: [bool; SimdOp::ALL.len()] = {
    let mut table = [false; SimdOp::ALL.len()];
    let mut i = 0;
    while i < table.len() {
        table[i] = is_numeric(SimdOp::ALL[i]);
        i += 1;
    }
    table
};

/// Forms that compute floating-point results or flags and so may raise a
/// numeric exception. SystemCpu stops them while MXCSR unmasks one.
#[inline]
pub(crate) fn numeric(op: SimdOp) -> bool {
    NUMERIC[op as usize]
}

fn gpr(index: usize) -> RtReg {
    GprIndex32::from_u8(index as u8).into()
}

fn read_gpr(state: &ProcessorState, index: usize) -> u32 {
    state.reg(gpr(index)).e().read()
}

fn write_gpr(state: &ProcessorState, index: usize, value: u32) {
    state.reg(gpr(index)).e().write(value);
}

fn mm(state: &ProcessorState, index: usize) -> [u8; 8] {
    mmx::read(&state.fpu, index).to_le_bytes()
}

fn set_mm(state: &mut ProcessorState, index: usize, bytes: &[u8]) {
    mmx::write(
        &mut state.fpu,
        index,
        u64::from_le_bytes(bytes[..8].try_into().unwrap()),
    );
}

/// Applies `f` to the 8 (MMX) or 16 (XMM) bytes of a register.
fn with_register(
    state: &mut ProcessorState,
    kind: SimdRegKind,
    index: usize,
    f: impl FnOnce(&mut [u8]),
) {
    if kind == SimdRegKind::Mm {
        let mut bytes = mm(state, index);
        f(&mut bytes);
        set_mm(state, index, &bytes);
    } else {
        f(&mut state.simd.xmm[index]);
    }
}

/// Integer and bitwise operations shared by the MMX and XMM forms.
#[derive(Debug, Clone, Copy)]
enum Int {
    Lane(Lane, usize),
    And,
    AndNot,
    Or,
    Xor,
    MulUnsignedDword,
    MultiplyAdd,
    SumAbsDiff,
    Pack(usize, bool),
    Unpack(usize, bool),
    Shift(usize, Shift),
}

const fn int_op(op: SimdOp) -> Option<Int> {
    use Int::*;
    use SimdOp::*;
    macro_rules! both {
        ($name:ident) => {
            paste::paste! { [<$name _Pq_Qq>] | [<$name _Vo_Wo>] }
        };
    }
    Some(match op {
        both!(PADDB) => Lane(lanes::Lane::Add, 1),
        both!(PADDW) => Lane(lanes::Lane::Add, 2),
        both!(PADDD) => Lane(lanes::Lane::Add, 4),
        both!(PADDQ) => Lane(lanes::Lane::Add, 8),
        both!(PSUBB) => Lane(lanes::Lane::Sub, 1),
        both!(PSUBW) => Lane(lanes::Lane::Sub, 2),
        both!(PSUBD) => Lane(lanes::Lane::Sub, 4),
        both!(PSUBQ) => Lane(lanes::Lane::Sub, 8),
        both!(PADDSB) => Lane(lanes::Lane::AddSigned, 1),
        both!(PADDSW) => Lane(lanes::Lane::AddSigned, 2),
        both!(PADDUSB) => Lane(lanes::Lane::AddUnsigned, 1),
        both!(PADDUSW) => Lane(lanes::Lane::AddUnsigned, 2),
        both!(PSUBSB) => Lane(lanes::Lane::SubSigned, 1),
        both!(PSUBSW) => Lane(lanes::Lane::SubSigned, 2),
        both!(PSUBUSB) => Lane(lanes::Lane::SubUnsigned, 1),
        both!(PSUBUSW) => Lane(lanes::Lane::SubUnsigned, 2),
        both!(PMULLW) => Lane(lanes::Lane::MulLow, 2),
        both!(PMULHW) => Lane(lanes::Lane::MulHigh, 2),
        both!(PMULHUW) => Lane(lanes::Lane::MulHighUnsigned, 2),
        both!(PCMPEQB) => Lane(lanes::Lane::CompareEqual, 1),
        both!(PCMPEQW) => Lane(lanes::Lane::CompareEqual, 2),
        both!(PCMPEQD) => Lane(lanes::Lane::CompareEqual, 4),
        both!(PCMPGTB) => Lane(lanes::Lane::CompareGreater, 1),
        both!(PCMPGTW) => Lane(lanes::Lane::CompareGreater, 2),
        both!(PCMPGTD) => Lane(lanes::Lane::CompareGreater, 4),
        both!(PAVGB) => Lane(lanes::Lane::Average, 1),
        both!(PAVGW) => Lane(lanes::Lane::Average, 2),
        both!(PMINUB) => Lane(lanes::Lane::MinUnsigned, 1),
        both!(PMAXUB) => Lane(lanes::Lane::MaxUnsigned, 1),
        both!(PMINSW) => Lane(lanes::Lane::MinSigned, 2),
        both!(PMAXSW) => Lane(lanes::Lane::MaxSigned, 2),
        both!(PAND) | ANDPS_Vo_Wo | ANDPD_Vo_Wo => And,
        both!(PANDN) | ANDNPS_Vo_Wo | ANDNPD_Vo_Wo => AndNot,
        both!(POR) | ORPS_Vo_Wo | ORPD_Vo_Wo => Or,
        both!(PXOR) | XORPS_Vo_Wo | XORPD_Vo_Wo => Xor,
        both!(PMULUDQ) => MulUnsignedDword,
        both!(PMADDWD) => MultiplyAdd,
        both!(PSADBW) => SumAbsDiff,
        both!(PACKSSWB) => Pack(2, false),
        both!(PACKSSDW) => Pack(4, false),
        both!(PACKUSWB) => Pack(2, true),
        PUNPCKLBW_Pq_Qd | PUNPCKLBW_Vo_Wo => Unpack(1, false),
        PUNPCKLWD_Pq_Qd | PUNPCKLWD_Vo_Wo => Unpack(2, false),
        PUNPCKLDQ_Pq_Qd | PUNPCKLDQ_Vo_Wo => Unpack(4, false),
        PUNPCKLQDQ_Vo_Wo => Unpack(8, false),
        both!(PUNPCKHBW) => Unpack(1, true),
        both!(PUNPCKHWD) => Unpack(2, true),
        both!(PUNPCKHDQ) => Unpack(4, true),
        PUNPCKHQDQ_Vo_Wo => Unpack(8, true),
        UNPCKLPS_Vo_Wo => Unpack(4, false),
        UNPCKHPS_Vo_Wo => Unpack(4, true),
        UNPCKLPD_Vo_Wo => Unpack(8, false),
        UNPCKHPD_Vo_Wo => Unpack(8, true),
        both!(PSRLW) => Shift(2, lanes::Shift::Right),
        both!(PSRLD) => Shift(4, lanes::Shift::Right),
        both!(PSRLQ) => Shift(8, lanes::Shift::Right),
        both!(PSRAW) => Shift(2, lanes::Shift::Arithmetic),
        both!(PSRAD) => Shift(4, lanes::Shift::Arithmetic),
        both!(PSLLW) => Shift(2, lanes::Shift::Left),
        both!(PSLLD) => Shift(4, lanes::Shift::Left),
        both!(PSLLQ) => Shift(8, lanes::Shift::Left),
        _ => return None,
    })
}

fn apply_int(dst: &mut [u8], src: &[u8], op: Int) {
    let src = &src[..dst.len()];
    match op {
        Int::Lane(lane, width) => lanes::lanes(dst, src, width, lane),
        Int::And => lanes::logic(dst, src, lanes::Logic::And),
        Int::AndNot => lanes::logic(dst, src, lanes::Logic::AndNot),
        Int::Or => lanes::logic(dst, src, lanes::Logic::Or),
        Int::Xor => lanes::logic(dst, src, lanes::Logic::Xor),
        Int::MulUnsignedDword => lanes::mul_unsigned_dword(dst, src),
        Int::MultiplyAdd => lanes::multiply_add(dst, src),
        Int::SumAbsDiff => lanes::sum_abs_diff(dst, src),
        Int::Pack(width, unsigned) => lanes::pack(dst, src, width, unsigned),
        Int::Unpack(width, high) => lanes::unpack(dst, src, width, high),
        // The count is the low quadword of the source.
        Int::Shift(width, kind) => lanes::shift(dst, width, kind, get(src, 0, 8)),
    }
}

/// The lane-by-lane floating-point forms other than ADD/SUB/MUL/DIV/SQRT,
/// which [`float::arith_lanes`] runs.
#[derive(Debug, Clone, Copy)]
enum Fp {
    Min,
    Max,
    Compare,
}

/// Floating-point forms as (operation, format, lanes); one lane is the
/// scalar form, which leaves the other lanes of the destination alone.
const fn fp_op(op: SimdOp) -> Option<Kind> {
    use SimdOp::*;
    macro_rules! forms {
        ($name:ident, $kind:ident($fp:expr)) => {
            paste::paste! {
                match op {
                    [<$name PS_Vo_Wo>] => return Some(Kind::$kind($fp, F32, 4)),
                    [<$name PD_Vo_Wo>] => return Some(Kind::$kind($fp, F64, 2)),
                    [<$name SS_Vd_Wd>] => return Some(Kind::$kind($fp, F32, 1)),
                    [<$name SD_Vq_Wq>] => return Some(Kind::$kind($fp, F64, 1)),
                    _ => {}
                }
            }
        };
    }
    forms!(ADD, Arith(Arith::Add));
    forms!(SUB, Arith(Arith::Sub));
    forms!(MUL, Arith(Arith::Mul));
    forms!(DIV, Arith(Arith::Div));
    forms!(SQRT, Arith(Arith::Sqrt));
    forms!(MIN, Fp(Fp::Min));
    forms!(MAX, Fp(Fp::Max));
    Some(match op {
        CMPPS_Vo_Wo_Ib => Kind::Fp(Fp::Compare, F32, 4),
        CMPPD_Vo_Wo_Ib => Kind::Fp(Fp::Compare, F64, 2),
        CMPSS_Vd_Wd_Ib => Kind::Fp(Fp::Compare, F32, 1),
        CMPSD_Vq_Wq_Ib => Kind::Fp(Fp::Compare, F64, 1),
        _ => return None,
    })
}

/// What `execute` does for a form, looked up once per instruction instead of
/// matching the form several times.
#[derive(Clone, Copy)]
enum Kind {
    Int(Int),
    Arith(Arith, Format, usize),
    Fp(Fp, Format, usize),
    Other,
}

#[derive(Clone, Copy)]
struct Plan {
    kind: Kind,
    reg: SimdRegKind,
    rm: SimdRegKind,
}

const PLANS: [Plan; SimdOp::ALL.len()] = {
    let mut plans = [Plan {
        kind: Kind::Other,
        reg: SimdRegKind::None,
        rm: SimdRegKind::None,
    }; SimdOp::ALL.len()];
    let mut i = 0;
    while i < plans.len() {
        let op = SimdOp::ALL[i];
        plans[i] = Plan {
            kind: match int_op(op) {
                Some(int) => Kind::Int(int),
                None => match fp_op(op) {
                    Some(kind) => kind,
                    None => Kind::Other,
                },
            },
            reg: op.reg_kind(),
            rm: op.rm_kind(),
        };
        i += 1;
    }
    plans
};

/// Every form but the memory stores (see [`store`]) and FXSAVE/FXRSTOR,
/// CLFLUSH and MASKMOVQ/MASKMOVDQU. `reg` is ModR/M.reg. `unmasked_ok` is
/// false on HarvardUME and FlatUME, which reject an LDMXCSR that unmasks a
/// numeric exception (#XM is not delivered); a numeric form is then
/// `Unimplemented` while MXCSR unmasks one anyway (set by the host through
/// [`SimdState::set_mxcsr`](super::SimdState::set_mxcsr)).
pub(crate) fn execute(
    state: &mut ProcessorState,
    op: SimdOp,
    reg: usize,
    rm: Rm,
    imm: u8,
    unmasked_ok: bool,
) -> Result<(), SimdFault> {
    use SimdOp::*;
    // MXCSR first: the common all-masked case skips the table lookup.
    if !unmasked_ok && !state.simd.all_masked() && numeric(op) {
        return Err(SimdFault::Unimplemented);
    }
    let plan = PLANS[op as usize];
    let mut src = [0u8; 16];
    let rm_reg = match rm {
        Rm::Mem(bytes) => {
            src[..bytes.len()].copy_from_slice(bytes);
            None
        }
        Rm::Reg(index) => {
            match plan.rm {
                SimdRegKind::Xmm => src = state.simd.xmm[index],
                SimdRegKind::Mm => src[..8].copy_from_slice(&mm(state, index)),
                SimdRegKind::Gpr => src[..4].copy_from_slice(&read_gpr(state, index).to_le_bytes()),
                SimdRegKind::None => {}
            }
            Some(index)
        }
    };
    // Any MMX register operand: TOP = 0 and all tags valid afterwards.
    let mmx = plan.reg == SimdRegKind::Mm || (plan.rm == SimdRegKind::Mm && rm_reg.is_some());
    let mut env = Env::new(state.simd.mxcsr);

    if let Kind::Int(int) = plan.kind {
        with_register(state, plan.reg, reg, |dst| apply_int(dst, &src, int));
    } else if let Kind::Arith(arith, f, n) = plan.kind {
        float::arith_lanes(f, &mut env, arith, &mut state.simd.xmm[reg], &src, n);
    } else if let Kind::Fp(fp, f, n) = plan.kind {
        let w = f.bytes();
        let dst = &mut state.simd.xmm[reg];
        for lane in 0..n {
            let (a, b) = (get(dst, lane, w), get(&src, lane, w));
            let r = match fp {
                Fp::Min => float::min_max(f, &mut env, false, a, b),
                Fp::Max => float::min_max(f, &mut env, true, a, b),
                Fp::Compare => {
                    if float::compare(f, &mut env, imm, a, b) {
                        u64::MAX
                    } else {
                        0
                    }
                }
            };
            put(dst, lane, w, r);
        }
    } else {
        match op {
            // Register moves and loads.
            MOVUPS_Vo_Wo | MOVUPD_Vo_Wo | MOVAPS_Vo_Wo | MOVAPD_Vo_Wo | MOVDQA_Vo_Wo
            | MOVDQU_Vo_Wo => state.simd.xmm[reg] = src,
            MOVUPS_Wo_Vo | MOVUPD_Wo_Vo | MOVAPS_Wo_Vo | MOVAPD_Wo_Vo | MOVDQA_Wo_Vo
            | MOVDQU_Wo_Vo => state.simd.xmm[rm_reg.unwrap()] = state.simd.xmm[reg],
            // A load clears the upper lanes; a register move merges.
            MOVSS_Vd_Wd | MOVSD_Vq_Wq => {
                let w = if op == MOVSS_Vd_Wd { 4 } else { 8 };
                if rm_reg.is_some() {
                    state.simd.xmm[reg][..w].copy_from_slice(&src[..w]);
                } else {
                    state.simd.xmm[reg] = src;
                }
            }
            MOVSS_Wd_Vd | MOVSD_Wq_Vq => {
                let w = if op == MOVSS_Wd_Vd { 4 } else { 8 };
                let value = state.simd.xmm[reg];
                state.simd.xmm[rm_reg.unwrap()][..w].copy_from_slice(&value[..w]);
            }
            MOVLPS_Vq_Mq | MOVLPD_Vq_Mq => state.simd.xmm[reg][..8].copy_from_slice(&src[..8]),
            MOVHPS_Vq_Mq | MOVHPD_Vq_Mq => state.simd.xmm[reg][8..].copy_from_slice(&src[..8]),
            MOVHLPS_Vq_Uq => state.simd.xmm[reg][..8].copy_from_slice(&src[8..]),
            MOVLHPS_Vq_Uq => state.simd.xmm[reg][8..].copy_from_slice(&src[..8]),
            MOVQ_Vo_Wq | MOVD_Vo_Ed | MOVQ2DQ_Vo_Nq => {
                let w = if op == MOVD_Vo_Ed { 4 } else { 8 };
                let mut value = [0; 16];
                value[..w].copy_from_slice(&src[..w]);
                state.simd.xmm[reg] = value;
            }
            MOVQ_Wq_Vq => {
                let mut value = [0; 16];
                value[..8].copy_from_slice(&state.simd.xmm[reg][..8]);
                state.simd.xmm[rm_reg.unwrap()] = value;
            }
            MOVD_Pq_Ed => set_mm(
                state,
                reg,
                &u64::from(u32::from_le_bytes(src[..4].try_into().unwrap())).to_le_bytes(),
            ),
            MOVQ_Pq_Qq | MOVDQ2Q_Pq_Uq => set_mm(state, reg, &src[..8]),
            MOVQ_Qq_Pq => {
                let value = mm(state, reg);
                set_mm(state, rm_reg.unwrap(), &value);
            }
            MOVD_Ed_Pd => write_gpr(state, rm_reg.unwrap(), mmx::read(&state.fpu, reg) as u32),
            MOVD_Ed_Vd => {
                let value = u32::from_le_bytes(state.simd.xmm[reg][..4].try_into().unwrap());
                write_gpr(state, rm_reg.unwrap(), value);
            }
            LDMXCSR_Md => {
                let value = u32::from_le_bytes(src[..4].try_into().unwrap());
                if value & !MXCSR_MASK != 0 {
                    return Err(SimdFault::GeneralProtection);
                }
                if !unmasked_ok && value & MXCSR_EXCEPTION_MASKS != MXCSR_EXCEPTION_MASKS {
                    return Err(SimdFault::Unimplemented);
                }
                state.simd.mxcsr = value;
            }
            EMMS => mmx::emms(&mut state.fpu),
            // Fences order nothing in PLUM's in-order memory model;
            // CLFLUSH only checks its operand (in the engine).
            LFENCE | MFENCE | SFENCE | CLFLUSH_Mb => {}

            // Masks, inserts, extracts and shuffles.
            MOVMSKPS_Gd_Uo => write_gpr(state, reg, lanes::sign_mask(&src, 4)),
            MOVMSKPD_Gd_Uo => write_gpr(state, reg, lanes::sign_mask(&src, 8)),
            PMOVMSKB_Gd_Nq => write_gpr(state, reg, lanes::sign_mask(&src[..8], 1)),
            PMOVMSKB_Gd_Uo => write_gpr(state, reg, lanes::sign_mask(&src, 1)),
            PEXTRW_Gd_Nq_Ib => write_gpr(state, reg, get(&src, usize::from(imm & 3), 2) as u32),
            PEXTRW_Gd_Uo_Ib => write_gpr(state, reg, get(&src, usize::from(imm & 7), 2) as u32),
            PINSRW_Pq_Ew_Ib | PINSRW_Vo_Ew_Ib => {
                let lane = usize::from(imm & if op == PINSRW_Pq_Ew_Ib { 3 } else { 7 });
                let value = get(&src, 0, 2);
                with_register(state, op.reg_kind(), reg, |dst| put(dst, lane, 2, value));
            }
            PSHUFW_Pq_Qq_Ib => with_register(state, SimdRegKind::Mm, reg, |dst| {
                lanes::shuffle(dst, &src, 2, 0, imm)
            }),
            PSHUFD_Vo_Wo_Ib => lanes::shuffle(&mut state.simd.xmm[reg], &src, 4, 0, imm),
            PSHUFHW_Vo_Wo_Ib | PSHUFLW_Vo_Wo_Ib => {
                let dst = &mut state.simd.xmm[reg];
                let (keep, offset) = if op == PSHUFHW_Vo_Wo_Ib {
                    (0..8, 4)
                } else {
                    (8..16, 0)
                };
                dst[keep.clone()].copy_from_slice(&src[keep]);
                lanes::shuffle(dst, &src, 2, offset, imm);
            }
            SHUFPS_Vo_Wo_Ib => {
                let dst = &mut state.simd.xmm[reg];
                let a = *dst;
                for i in 0..4 {
                    let pick = usize::from(imm >> (2 * i)) & 3;
                    let from = if i < 2 { &a[..] } else { &src[..] };
                    put(dst, i, 4, get(from, pick, 4));
                }
            }
            SHUFPD_Vo_Wo_Ib => {
                let dst = &mut state.simd.xmm[reg];
                let a = *dst;
                put(dst, 0, 8, get(&a, usize::from(imm & 1), 8));
                put(dst, 1, 8, get(&src, usize::from((imm >> 1) & 1), 8));
            }

            // Shifts by an immediate: the r/m register is the destination.
            PSRLW_Nq_Ib | PSRAW_Nq_Ib | PSLLW_Nq_Ib | PSRLD_Nq_Ib | PSRAD_Nq_Ib | PSLLD_Nq_Ib
            | PSRLQ_Nq_Ib | PSLLQ_Nq_Ib | PSRLW_Uo_Ib | PSRAW_Uo_Ib | PSLLW_Uo_Ib | PSRLD_Uo_Ib
            | PSRAD_Uo_Ib | PSLLD_Uo_Ib | PSRLQ_Uo_Ib | PSLLQ_Uo_Ib => {
                let name = op.name().as_bytes();
                let width = match name[4] {
                    b'W' => 2,
                    b'D' => 4,
                    _ => 8,
                };
                let kind = match &name[..4] {
                    b"PSRL" => Shift::Right,
                    b"PSRA" => Shift::Arithmetic,
                    _ => Shift::Left,
                };
                with_register(state, op.rm_kind(), rm_reg.unwrap(), |dst| {
                    lanes::shift(dst, width, kind, u64::from(imm))
                });
            }
            PSRLDQ_Uo_Ib => lanes::shift_bytes(&mut state.simd.xmm[rm_reg.unwrap()], false, imm),
            PSLLDQ_Uo_Ib => lanes::shift_bytes(&mut state.simd.xmm[rm_reg.unwrap()], true, imm),

            // RCP and RSQRT ignore MXCSR.
            RCPPS_Vo_Wo | RSQRTPS_Vo_Wo | RCPSS_Vd_Wd | RSQRTSS_Vd_Wd => {
                let sqrt = matches!(op, RSQRTPS_Vo_Wo | RSQRTSS_Vd_Wd);
                let n = if matches!(op, RCPPS_Vo_Wo | RSQRTPS_Vo_Wo) {
                    4
                } else {
                    1
                };
                let dst = &mut state.simd.xmm[reg];
                for lane in 0..n {
                    let value = float::reciprocal(get(&src, lane, 4) as u32, sqrt);
                    put(dst, lane, 4, u64::from(value));
                }
            }

            // Compares into EFLAGS: ZF, PF, CF; OF, SF and AF cleared.
            COMISS_Vd_Wd | UCOMISS_Vd_Wd | COMISD_Vq_Wq | UCOMISD_Vq_Wq => {
                let f = if matches!(op, COMISS_Vd_Wd | UCOMISS_Vd_Wd) {
                    F32
                } else {
                    F64
                };
                let quiet = matches!(op, UCOMISS_Vd_Wd | UCOMISD_Vq_Wq);
                let w = f.bytes();
                let a = get(&state.simd.xmm[reg], 0, w);
                let (zf, pf, cf) = float::compare_flags(f, &mut env, quiet, a, get(&src, 0, w));
                state.flags_mut().set_compare_result(cf, zf, pf);
            }

            // Conversions.
            CVTPI2PS_Vq_Qq | CVTPI2PD_Vo_Qq | CVTDQ2PS_Vo_Wo | CVTDQ2PD_Vo_Wq | CVTSI2SS_Vd_Ed
            | CVTSI2SD_Vq_Ed => {
                let (f, n) = match op {
                    CVTPI2PS_Vq_Qq => (F32, 2),
                    CVTDQ2PS_Vo_Wo => (F32, 4),
                    CVTSI2SS_Vd_Ed => (F32, 1),
                    CVTSI2SD_Vq_Ed => (F64, 1),
                    _ => (F64, 2),
                };
                let dst = &mut state.simd.xmm[reg];
                for lane in 0..n {
                    let value = get(&src, lane, 4) as u32 as i32;
                    put(dst, lane, f.bytes(), float::from_i32(f, &mut env, value));
                }
            }
            CVTPS2PI_Pq_Wq | CVTTPS2PI_Pq_Wq | CVTPD2PI_Pq_Wo | CVTTPD2PI_Pq_Wo => {
                let f = if matches!(op, CVTPS2PI_Pq_Wq | CVTTPS2PI_Pq_Wq) {
                    F32
                } else {
                    F64
                };
                let truncate = matches!(op, CVTTPS2PI_Pq_Wq | CVTTPD2PI_Pq_Wo);
                let mut value = [0u8; 8];
                for lane in 0..2 {
                    let x = float::to_i32(f, &mut env, get(&src, lane, f.bytes()), truncate);
                    put(&mut value, lane, 4, u64::from(x));
                }
                set_mm(state, reg, &value);
            }
            CVTSS2SI_Gd_Wd | CVTTSS2SI_Gd_Wd | CVTSD2SI_Gd_Wq | CVTTSD2SI_Gd_Wq => {
                let f = if matches!(op, CVTSS2SI_Gd_Wd | CVTTSS2SI_Gd_Wd) {
                    F32
                } else {
                    F64
                };
                let truncate = matches!(op, CVTTSS2SI_Gd_Wd | CVTTSD2SI_Gd_Wq);
                let value = float::to_i32(f, &mut env, get(&src, 0, f.bytes()), truncate);
                write_gpr(state, reg, value);
            }
            CVTPS2DQ_Vo_Wo | CVTTPS2DQ_Vo_Wo | CVTPD2DQ_Vo_Wo | CVTTPD2DQ_Vo_Wo => {
                let (f, n) = if matches!(op, CVTPS2DQ_Vo_Wo | CVTTPS2DQ_Vo_Wo) {
                    (F32, 4)
                } else {
                    (F64, 2)
                };
                let truncate = matches!(op, CVTTPS2DQ_Vo_Wo | CVTTPD2DQ_Vo_Wo);
                let mut value = [0u8; 16];
                for lane in 0..n {
                    let x = float::to_i32(f, &mut env, get(&src, lane, f.bytes()), truncate);
                    put(&mut value, lane, 4, u64::from(x));
                }
                state.simd.xmm[reg] = value;
            }
            CVTPS2PD_Vo_Wq | CVTSS2SD_Vq_Wd => {
                let n = if op == CVTPS2PD_Vo_Wq { 2 } else { 1 };
                let dst = &mut state.simd.xmm[reg];
                for lane in 0..n {
                    let value = float::convert(F32, F64, &mut env, get(&src, lane, 4));
                    put(dst, lane, 8, value);
                }
            }
            CVTPD2PS_Vo_Wo | CVTSD2SS_Vd_Wq => {
                let packed = op == CVTPD2PS_Vo_Wo;
                let dst = &mut state.simd.xmm[reg];
                if packed {
                    *dst = [0; 16];
                }
                for lane in 0..if packed { 2 } else { 1 } {
                    let value = float::convert(F64, F32, &mut env, get(&src, lane, 8));
                    put(dst, lane, 4, value);
                }
            }
            _ => unreachable!("{op:?} is handled by the engine"),
        }
    }
    state.simd.mxcsr |= env.flags;
    if mmx {
        mmx::enter(&mut state.fpu);
    }
    Ok(())
}

/// A memory form that writes its operand: returns the first `op.mem_size()`
/// bytes. The engine has already checked the whole destination, so the store
/// cannot fault after the MMX state changes.
pub(crate) fn store(state: &mut ProcessorState, op: SimdOp, reg: usize) -> [u8; 16] {
    use SimdOp::*;
    let mut bytes = [0; 16];
    match op {
        MOVUPS_Wo_Vo | MOVUPD_Wo_Vo | MOVAPS_Wo_Vo | MOVAPD_Wo_Vo | MOVDQA_Wo_Vo | MOVDQU_Wo_Vo
        | MOVNTPS_Mo_Vo | MOVNTPD_Mo_Vo | MOVNTDQ_Mo_Vo | MOVSS_Wd_Vd | MOVSD_Wq_Vq
        | MOVQ_Wq_Vq | MOVLPS_Mq_Vq | MOVLPD_Mq_Vq | MOVD_Ed_Vd => bytes = state.simd.xmm[reg],
        MOVHPS_Mq_Vq | MOVHPD_Mq_Vq => bytes[..8].copy_from_slice(&state.simd.xmm[reg][8..]),
        MOVNTQ_Mq_Pq | MOVQ_Qq_Pq | MOVD_Ed_Pd => {
            bytes[..8].copy_from_slice(&mm(state, reg));
            mmx::enter(&mut state.fpu);
        }
        MOVNTI_Md_Gd => bytes[..4].copy_from_slice(&read_gpr(state, reg).to_le_bytes()),
        STMXCSR_Md => bytes[..4].copy_from_slice(&state.simd.mxcsr.to_le_bytes()),
        _ => unreachable!("{op:?} is not a store"),
    }
    bytes
}

/// MASKMOVQ/MASKMOVDQU: the bytes of ModR/M.reg to store at seg:(E)DI and
/// the mask of the bytes to store (the sign bits of the r/m register's
/// bytes). The engine checks the whole 8/16 bytes even with an all-zero mask
/// and stores only the selected bytes.
pub(crate) fn mask_move(
    state: &mut ProcessorState,
    op: SimdOp,
    reg: usize,
    rm: usize,
) -> ([u8; 16], u16) {
    let mut data = [0; 16];
    let mask = if op == SimdOp::MASKMOVQ_Pq_Nq {
        data[..8].copy_from_slice(&mm(state, reg));
        let mask = lanes::sign_mask(&mm(state, rm), 1) as u16;
        mmx::enter(&mut state.fpu);
        mask
    } else {
        data = state.simd.xmm[reg];
        lanes::sign_mask(&state.simd.xmm[rm], 1) as u16
    };
    (data, mask)
}
