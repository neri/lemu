//! Register-only x87 secondary dispatch. Instruction pointers remain engine-owned.
use core::cmp::Ordering;

use ir86::encoding::CC;

use super::ops::{arithmetic, square_root};
use super::uop::{FpuMinorUop, FpuOp, POP};
use super::{DENORMAL, FpuState, FpuValue, INDEFINITE, INVALID, Raw80, UnsupportedFpu};
use crate::cpu::ProcessorState;
use crate::gpr::PartialRegister;

pub(crate) fn compare(state: &mut ProcessorState, rhs: FpuValue, quiet: bool, integer_flags: bool) {
    let lhs = state.fpu.read(0).raw();
    let rhs = rhs.raw();
    let unordered = lhs.is_nan() || rhs.is_nan();
    let order = if unordered {
        Ordering::Equal
    } else {
        lhs.compare(rhs)
    };
    let fpu = &mut state.fpu;
    if lhs.is_denormal() || rhs.is_denormal() {
        fpu.status |= DENORMAL;
    }
    if (unordered && !quiet) || lhs.is_signaling_nan() || rhs.is_signaling_nan() {
        fpu.status |= INVALID;
    }
    fpu.status &= !0x200;
    let less = unordered || order == Ordering::Less;
    let equal = unordered || order == Ordering::Equal;
    if integer_flags {
        state.flags_mut().set_compare_result(less, equal, unordered);
    } else {
        fpu.status = (fpu.status & !0x4500)
            | if less { 0x100 } else { 0 }
            | if unordered { 0x400 } else { 0 }
            | if equal { 0x4000 } else { 0 };
    }
}

// Retain one secondary dispatch boundary per engine. Sharing source must not
// add another call for every register instruction.
#[inline(always)]
pub(crate) fn execute(state: &mut ProcessorState, op: FpuMinorUop) -> Result<(), UnsupportedFpu> {
    use FpuOp::*;
    match op.op {
        Unsupported => return Err(UnsupportedFpu),
        Wait | Nop => {} // All numeric exceptions are masked in this backend.
        Init => state.fpu = FpuState::default(),
        Clear => state.fpu.status &= !0x80ff,
        StoreStatusAx => state.ax().write(state.fpu.status_word()),
        LoadRegister => {
            let value = state.fpu.read(op.arg);
            state.fpu.push(value);
        }
        StoreRegister => {
            let value = state.fpu.read(0);
            state.fpu.set(op.arg & 7, value);
            if op.arg & POP != 0 {
                state.fpu.pop();
            }
        }
        ConditionalMove => {
            let cc = match (op.arg >> 3) & 3 {
                0 => CC::C,
                1 => CC::Z,
                2 => CC::BE,
                _ => CC::P,
            };
            let mut take = state.flags_mut().eval_cc(cc);
            take ^= op.arg & 0x20 != 0;
            let fpu = &mut state.fpu;
            fpu.status &= !0x200;
            let source = fpu.st(op.arg & 7);
            if fpu.st(0).is_none() || source.is_none() {
                fpu.status |= INVALID | 0x40;
                fpu.set(0, FpuValue::Raw80(INDEFINITE));
            } else if take {
                fpu.set(0, source.unwrap());
            }
        }
        Constant => {
            // Rounded positive 66-bit x87 constants. No PE is raised, and
            // PC does not affect constant loads; only RC selects the last bit.
            let down = matches!(state.fpu.rc(), 1 | 3);
            let up = state.fpu.rc() == 2;
            let (sign_exponent, significand) = match op.arg {
                0 => (0x4000, 0xd49a_784b_cd1b_8afeu64 + u64::from(up)), // log2(10)
                1 => (0x3fff, 0xb8aa_3b29_5c17_f0bcu64 - u64::from(down)), // log2(e)
                2 => (0x4000, 0xc90f_daa2_2168_c235u64 - u64::from(down)), // pi
                3 => (0x3ffd, 0x9a20_9a84_fbcf_f799u64 - u64::from(down)), // log10(2)
                _ => (0x3ffe, 0xb172_17f7_d1cf_79acu64 - u64::from(down)), // ln(2)
            };
            state.fpu.push(FpuValue::Raw80(Raw80 {
                significand,
                sign_exponent,
            }));
        }
        Exchange => {
            let a = state.fpu.read(0);
            let b = state.fpu.read(op.arg);
            state.fpu.set(0, b);
            state.fpu.set(op.arg, a);
        }
        Free => state.fpu.free(op.arg),
        IncrementTop | DecrementTop => {
            let fpu = &mut state.fpu;
            fpu.top = (fpu.top + if op.op == IncrementTop { 1 } else { 7 }) & 7;
            fpu.status &= !0x200;
        }
        ArithmeticRegister => {
            if !state.fpu.supports_arithmetic() {
                return Err(UnsupportedFpu);
            }
            let index = (op.arg >> 3) & 7;
            let destination = if op.arg & 0x80 != 0 { index } else { 0 };
            let source = state.fpu.read(if op.arg & 0x80 != 0 { 0 } else { index });
            let lhs = state.fpu.read(destination);
            let (value, flags) = arithmetic(lhs, source, op.arg & 7, state.fpu.control);
            state.fpu.status = (state.fpu.status & !0x200) | flags;
            state.fpu.set(destination, value);
            if op.arg & 0x40 != 0 {
                state.fpu.pop();
            }
        }
        CompareRegister => {
            let value = state.fpu.read(op.arg & 7);
            compare(state, value, op.arg & 0x20 != 0, op.arg & 0x40 != 0);
            for _ in 0..((op.arg >> 3) & 3) {
                state.fpu.pop();
            }
        }
        Sign | Abs => {
            let value = state.fpu.read(0).sign(op.op == Abs);
            state.fpu.set(0, value);
        }
        Zero | One => state
            .fpu
            .push(FpuValue::F32(if op.op == Zero { 0.0 } else { 1.0 })),
        Test => compare(state, FpuValue::F32(0.0), false, false),
        Examine => {
            let fpu = &mut state.fpu;
            let raw = fpu.registers[fpu.physical(0)].raw();
            let class = if fpu.st(0).is_none() {
                0x4100
            } else if !raw.is_supported() {
                0
            } else if raw.is_nan() {
                0x100
            } else if raw.is_infinite() {
                0x500
            } else if raw.is_zero() {
                0x4000
            } else if raw.is_denormal() {
                0x4400
            } else {
                0x400
            };
            fpu.status = (fpu.status & !0x4700) | class | if raw.is_negative() { 0x200 } else { 0 };
        }
        Round => {
            let fpu = &mut state.fpu;
            let (value, flags) = fpu.read(0).raw().round_integer(fpu.rc());
            fpu.status = (fpu.status & !0x200) | flags;
            fpu.set(0, FpuValue::Raw80(value));
        }
        Remainder => {
            let fpu = &mut state.fpu;
            let empty = fpu.st(0).is_none() || fpu.st(1).is_none();
            let input = fpu.read(0);
            let modulus = fpu.read(1);
            let (result, flags, condition) = if empty {
                (FpuValue::Raw80(INDEFINITE), INVALID, 0)
            } else {
                crate::fpu::remainder::remainder(input, modulus, op.arg != 0)
            };
            fpu.status = (fpu.status & !0x4700) | flags | condition;
            fpu.set(0, result);
        }
        Exp2m1 | LogMultiply => {
            let fpu = &mut state.fpu;
            if fpu.rc() != 0 {
                return Err(UnsupportedFpu);
            }
            // Check the documented domain before reads can update stack flags.
            if let Some(input) = fpu.st(0) {
                let raw = input.raw();
                let in_range = if op.op == Exp2m1 {
                    crate::fpu::logarithm::exp_in_range(raw)
                } else {
                    op.arg == 0 || crate::fpu::logarithm::log1p_in_range(raw)
                };
                if !in_range {
                    return Err(UnsupportedFpu);
                }
            }
            let empty = fpu.st(0).is_none() || (op.op == LogMultiply && fpu.st(1).is_none());
            let input = fpu.read(0);
            let (result, flags) = if op.op == Exp2m1 {
                crate::fpu::logarithm::exp2m1(input)
            } else {
                let y = fpu.read(1);
                if empty {
                    (FpuValue::Raw80(INDEFINITE), INVALID)
                } else {
                    crate::fpu::logarithm::log_multiply(input, y, op.arg != 0)
                }
            };
            fpu.status = (fpu.status & !0x200) | flags;
            if op.op == LogMultiply {
                fpu.set(1, result);
                fpu.pop();
            } else {
                fpu.set(0, result);
            }
        }
        Scale => {
            let fpu = &mut state.fpu;
            let empty = fpu.st(0).is_none() || fpu.st(1).is_none();
            let input = fpu.read(0);
            let power = fpu.read(1);
            let (result, flags) = if empty {
                (FpuValue::Raw80(INDEFINITE), INVALID)
            } else {
                crate::fpu::exponent::scale(input, power, fpu.control)
            };
            fpu.status = (fpu.status & !0x200) | flags;
            fpu.set(0, result);
        }
        Extract => {
            let fpu = &mut state.fpu;
            let overflow = fpu.st(7).is_some();
            if overflow || fpu.st(0).is_none() {
                fpu.status =
                    (fpu.status & !0x200) | INVALID | 0x40 | if overflow { 0x200 } else { 0 };
                fpu.set(0, FpuValue::Raw80(INDEFINITE));
                fpu.push(FpuValue::Raw80(INDEFINITE));
            } else {
                let input = fpu.read(0);
                let (exponent, significand, flags) = crate::fpu::exponent::extract(input);
                fpu.status = (fpu.status & !0x200) | flags;
                fpu.set(0, exponent);
                fpu.push(significand);
            }
        }
        Trig | Atan => {
            // libm exposes no guest-directed rounding. Reject before even
            // reading an empty operand, so unsupported modes cannot mutate state.
            if state.fpu.rc() != 0 {
                return Err(UnsupportedFpu);
            }
            let fpu = &mut state.fpu;
            if op.op == Atan {
                let empty = fpu.st(0).is_none() || fpu.st(1).is_none();
                let x = fpu.read(0);
                let y = fpu.read(1);
                let (result, flags) = if empty {
                    (FpuValue::Raw80(INDEFINITE), INVALID)
                } else {
                    crate::fpu::trig::atan(y, x)
                };
                fpu.status = (fpu.status & !0x200) | flags;
                fpu.set(1, result);
                fpu.pop();
            } else {
                let pair = op.arg >= 2;
                let overflow = pair && fpu.st(7).is_some();
                if overflow || fpu.st(0).is_none() {
                    // A masked stack fault writes indefinite to both
                    // destinations. Stack overflow takes precedence over C2.
                    fpu.status =
                        (fpu.status & !0x600) | INVALID | 0x40 | if overflow { 0x200 } else { 0 };
                    fpu.set(0, FpuValue::Raw80(INDEFINITE));
                    if pair {
                        fpu.push(FpuValue::Raw80(INDEFINITE));
                    }
                } else {
                    let input = fpu.read(0);
                    match crate::fpu::trig::unary(input, op.arg) {
                        crate::fpu::trig::UnaryResult::OutOfRange => fpu.status |= 0x400,
                        crate::fpu::trig::UnaryResult::Value {
                            first,
                            pushed,
                            flags,
                        } => {
                            fpu.status = (fpu.status & !0x600) | flags;
                            fpu.set(0, first);
                            if let Some(value) = pushed {
                                fpu.push(value);
                            }
                        }
                    }
                }
            }
        }
        Sqrt => {
            if !state.fpu.supports_arithmetic() {
                return Err(UnsupportedFpu);
            }
            let fpu = &mut state.fpu;
            let input = fpu.read(0);
            let (result, flags) = square_root(input, fpu.control);
            fpu.status = (fpu.status & !0x200) | flags;
            fpu.set(0, result);
        }
        _ => return Err(UnsupportedFpu),
    }
    Ok(())
}
