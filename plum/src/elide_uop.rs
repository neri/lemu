//! Map an engine’s register Uop to its flag-free equivalent.
use super::uop::{BinaryRmwKind, Uop, ValueShift};
use crate::prelude::Offset32;

pub(super) fn rewrite(slot: &mut Uop) {
    *slot = match *slot {
        Uop::AddI(r, v) => Uop::ValueAddI(r, v),
        Uop::SubI(r, v) => Uop::ValueSubI(r, v),
        Uop::DecR(r) => Uop::LeaBD(r, r, Offset32(u32::MAX)),
        Uop::IMulR(d, s) => Uop::ValueMulR(d, s),
        Uop::IMulRI(d, s, v) => Uop::ValueMulRI(d, s, v),
        Uop::ShlI(d, n) => Uop::ValueShift(ValueShift::Shl, d, Some(n)),
        Uop::ShlRCl(d) => Uop::ValueShift(ValueShift::Shl, d, None),
        Uop::ShrI(d, n) => Uop::ValueShift(ValueShift::Shr, d, Some(n)),
        Uop::ShrRCl(d) => Uop::ValueShift(ValueShift::Shr, d, None),
        Uop::SarI(d, n) => Uop::ValueShift(ValueShift::Sar, d, Some(n)),
        Uop::SarRCl(d) => Uop::ValueShift(ValueShift::Sar, d, None),
        Uop::NegR(r) => Uop::ValueNeg(r),
        Uop::SubR(d, s) => Uop::ValueSubR(d, s),
        Uop::AndR(d, s) => Uop::ValueAndR(d, s),
        Uop::OrR(d, s) => Uop::ValueOrR(d, s),
        Uop::OrI(d, v) => Uop::ValueOrI(d, v),
        Uop::XorI(d, v) => Uop::ValueXorI(d, v),
        Uop::AdcI(d, v) => Uop::ValueImm(BinaryRmwKind::Adc, d, v),
        Uop::AdcR(d, s) => Uop::ValueReg(BinaryRmwKind::Adc, d, s),
        Uop::SbbI(d, v) => Uop::ValueImm(BinaryRmwKind::Sbb, d, v),
        Uop::SbbR(d, s) => Uop::ValueReg(BinaryRmwKind::Sbb, d, s),
        Uop::IncR(r) => Uop::LeaBD(r, r, Offset32(1)),
        Uop::AddR(d, s) => Uop::ValueAddR(d, s),
        Uop::AndI(d, v) => Uop::ValueAndI(d, v),
        Uop::XorR(d, s) => Uop::ValueXorR(d, s),
        _ => return,
    };
}
