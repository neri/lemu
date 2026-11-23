//! Arithmetic Logic Unit (ALU)
mod decimal;
pub(crate) mod division;

use core::ops::{BitAnd, BitOr, BitXor};

use paste::paste;

use crate::cpu::{ProcessorState, RtReg, RtReg8};
use crate::gpr::PartialRegister;
use crate::prelude::*;

/// Arithmetic Logic Unit (ALU)
#[repr(transparent)]
pub struct Alu<'a>(pub &'a mut ProcessorState);

macro_rules! add_sub {
    ($name:ident, $method:ident, $lazy_op:ident, $op_size:expr) => {
        paste! {
            #[inline]
            pub fn [< $name $op_size >] (&mut self, dst: [< u $op_size >], src: [< u $op_size >]) -> [< u $op_size >] {
                let result = dst.$method(src);
                self.0.reset_flags_with_zf(result == 0);
                self.0.set_lazy_op(LazyOp::[< $lazy_op $op_size >](dst, src));
                result
            }
        }
    };
    ($name:ident, $method:ident, $lazy_op:ident) => {
        add_sub!($name, $method, $lazy_op, 8);
        add_sub!($name, $method, $lazy_op, 16);
        add_sub!($name, $method, $lazy_op, 32);
    }
}

macro_rules! adc_sbb {
    ($name:ident, $method:ident, $lazy_op:ident, $op_size:expr) => {
        paste! {
            #[inline]
            pub fn [< $name $op_size >] (&mut self, dst: [< u $op_size >], src: [< u $op_size >]) -> [< u $op_size >] {
                let cf = self.0.flags_mut().recompute_cf();
                let result = dst.$method(src).$method(cf as [< u $op_size >]);
                self.0.reset_flags_with_zf(result == 0);
                self.0.set_lazy_op(LazyOp::[< $lazy_op $op_size >](dst, src, cf));
                result
            }
        }
    };
    ($name:ident, $method:ident, $lazy_op:ident) => {
        adc_sbb!($name, $method, $lazy_op, 8);
        adc_sbb!($name, $method, $lazy_op, 16);
        adc_sbb!($name, $method, $lazy_op, 32);
    }
}

macro_rules! inc_dec {
    ($name:ident, $method:ident, $lazy_op:ident, $op_size:expr) => {
        paste! {
            #[inline]
            pub fn [< $name $op_size >] (&mut self, dst: [< u $op_size >]) -> [< u $op_size >] {
                let cf = self.0.flags_mut().recompute_cf();
                let result = dst.$method(1);
                self.0.adjust_after_inc_dec(cf, result == 0);
                self.0.set_lazy_op(LazyOp::[< $lazy_op $op_size >](dst, cf));
                result
            }
        }
    };
    ($name:ident, $method:ident, $lazy_op:ident) => {
        inc_dec!($name, $method, $lazy_op, 8);
        inc_dec!($name, $method, $lazy_op, 16);
        inc_dec!($name, $method, $lazy_op, 32);
    };
}

macro_rules! logic_op {
    ($name:ident, $method:ident, $lazy_op:ident, $op_size:expr) => {
        paste! {
            #[inline]
            pub fn [< $name $op_size >] (&mut self, dst: [< u $op_size >], src: [< u $op_size >]) -> [< u $op_size >] {
                let result = dst.$method(src);
                self.0.adjust_after_logic_op(result == 0);
                self.0.set_lazy_op(LazyOp::[< $lazy_op $op_size >](dst, src));
                result
            }
        }
    };
    ($name:ident, $method:ident, $lazy_op:ident) => {
        logic_op!($name, $method, $lazy_op, 8);
        logic_op!($name, $method, $lazy_op, 16);
        logic_op!($name, $method, $lazy_op, 32);
    }
}

macro_rules! shift_op {
    ($name:ident, $method:ident, $lazy_op:ident, $op_size:expr) => {
        paste! {
            #[inline]
            pub fn [< $name $op_size >] (&mut self, dst: [< u $op_size >], count: u8) -> [< u $op_size >] {
                let count = count & 0x1f;
                if count > 0 {
                    let result = $method(dst as u32, count, $op_size) as [< u $op_size >];
                    self.0.reset_flags_with_zf(result == 0);
                    self.0.set_lazy_op(LazyOp::[< $lazy_op $op_size >](dst, count));
                    result
                } else {
                    dst
                }
            }
        }
    };
    ($name:ident, $method:ident, $lazy_op:ident) => {
        shift_op!($name, $method, $lazy_op, 8);
        shift_op!($name, $method, $lazy_op, 16);
        shift_op!($name, $method, $lazy_op, 32);
    }
}

impl<'a> Alu<'a> {
    #[inline]
    pub fn new(state: &'a mut ProcessorState) -> Self {
        Self(state)
    }
}

impl Alu<'_> {
    add_sub!(add, wrapping_add, Add);
    add_sub!(sub, wrapping_sub, Sub);

    adc_sbb!(adc, wrapping_add, Adc);
    adc_sbb!(sbb, wrapping_sub, Sbb);

    inc_dec!(inc, wrapping_add, Inc);
    inc_dec!(dec, wrapping_sub, Dec);

    logic_op!(and, bitand, And);
    logic_op!(or, bitor, Or);
    logic_op!(xor, bitxor, Xor);

    #[inline]
    pub(crate) fn xadd8(&mut self, rd: RtReg8, rs: RtReg8) {
        let dst = self.0.reg8(rd).read();
        let src = self.0.reg8(rs).read();
        let sum = self.add8(dst, src);
        self.0.reg8(rs).write(dst);
        self.0.reg8(rd).write(sum);
    }

    #[inline]
    pub(crate) fn xadd16(&mut self, rd: RtReg, rs: RtReg) {
        let dst = self.0.reg(rd).w().read();
        let src = self.0.reg(rs).w().read();
        let sum = self.add16(dst, src);
        self.0.reg(rs).w().write(dst);
        self.0.reg(rd).w().write(sum);
    }

    #[inline]
    pub(crate) fn xadd32(&mut self, rd: RtReg, rs: RtReg) {
        let dst = self.0.reg(rd).e().read();
        let src = self.0.reg(rs).e().read();
        let sum = self.add32(dst, src);
        self.0.reg(rs).e().write(dst);
        self.0.reg(rd).e().write(sum);
    }

    pub fn mul8(&mut self, dst: u8, src: u8) -> u16 {
        let result = (dst as u16).wrapping_mul(src as u16);
        self.0.flags_mut().adjust_after_mul();
        self.0.set_lazy_op(LazyOp::Mul8(dst, src));
        result
    }

    pub fn mul16(&mut self, dst: u16, src: u16) -> u32 {
        let result = (dst as u32).wrapping_mul(src as u32);
        self.0.flags_mut().adjust_after_mul();
        self.0.set_lazy_op(LazyOp::Mul16(dst, src));
        result
    }

    pub fn mul32(&mut self, dst: u32, src: u32) -> u64 {
        let result = (dst as u64).wrapping_mul(src as u64);
        self.0.flags_mut().adjust_after_mul();
        self.0.set_lazy_op(LazyOp::Mul32(dst, src));
        result
    }

    pub fn imul8(&mut self, dst: i8, src: i8) -> i8 {
        let result = dst.wrapping_mul(src);
        self.0.flags_mut().adjust_after_mul();
        self.0.set_lazy_op(LazyOp::IMul8(dst, src));
        result
    }

    pub fn imul16(&mut self, dst: i16, src: i16) -> i16 {
        let result = dst.wrapping_mul(src);
        self.0.flags_mut().adjust_after_mul();
        self.0.set_lazy_op(LazyOp::IMul16(dst, src));
        result
    }

    pub fn imul32(&mut self, dst: i32, src: i32) -> i32 {
        let result = dst.wrapping_mul(src);
        self.0.flags_mut().adjust_after_mul();
        self.0.set_lazy_op(LazyOp::IMul32(dst, src));
        result
    }

    shift_op!(shl, shift_left, Shl);
    shift_op!(shr, shift_right, Shr);

    pub fn sar8(&mut self, dst: u8, count: u8) -> u8 {
        let count = count & 0x1f;
        if count > 0 {
            let dst = dst as i8;
            let result = shift_arithmetic_right(dst as u32, count, 8) as u8;
            self.0.reset_flags_with_zf(result == 0);
            self.0.set_lazy_op(LazyOp::Sar8(dst, count));
            result
        } else {
            dst
        }
    }

    pub fn sar16(&mut self, dst: u16, count: u8) -> u16 {
        let count = count & 0x1f;
        if count > 0 {
            let dst = dst as i16;
            let result = shift_arithmetic_right(dst as u32, count, 16) as u16;
            self.0.reset_flags_with_zf(result == 0);
            self.0.set_lazy_op(LazyOp::Sar16(dst, count));
            result
        } else {
            dst
        }
    }

    pub fn sar32(&mut self, dst: u32, count: u8) -> u32 {
        let count = count & 0x1f;
        if count > 0 {
            let dst = dst as i32;
            let result = shift_arithmetic_right(dst as u32, count, 32) as u32;
            self.0.reset_flags_with_zf(result == 0);
            self.0.set_lazy_op(LazyOp::Sar32(dst, count));
            result
        } else {
            dst
        }
    }
}

/// x86 masks counts to five bits, even for byte and word operands.
pub(crate) fn shift_left(value: u32, count: u8, bits: u32) -> u32 {
    (((value as u64) << count) & width_mask(bits)) as u32
}
pub(crate) fn shift_right(value: u32, count: u8, _bits: u32) -> u32 {
    value >> count
}
pub(crate) fn shift_arithmetic_right(value: u32, count: u8, bits: u32) -> u32 {
    let signed = ((value << (32 - bits)) as i32) >> (32 - bits);
    (signed >> count) as u32 & width_mask(bits) as u32
}
fn width_mask(bits: u32) -> u64 {
    (1u64 << bits) - 1
}

impl Alu<'_> {
    /// Rotate while preserving every flag other than CF/OF.
    #[inline]
    fn rotate(&mut self, value: u32, count: u8, bits: u32, right: bool, carry: bool) -> u32 {
        let masked = u32::from(count & 31);
        let width = bits + u32::from(carry);
        let shift = masked % width;
        // A full ROL/ROR still updates CF. A full RCL/RCR preserves it.
        if masked == 0 || (carry && shift == 0) {
            return value;
        }
        let mut ring = u64::from(value);
        if carry {
            ring |= u64::from(self.0.flags_mut().recompute_cf()) << bits;
        }
        let rotated = if right {
            (ring >> shift) | (ring << (width - shift))
        } else {
            (ring << shift) | (ring >> (width - shift))
        };
        let result = (rotated & width_mask(bits)) as u32;
        let cf = if carry {
            rotated & (1 << bits) != 0
        } else if right {
            result & (1 << (bits - 1)) != 0
        } else {
            result & 1 != 0
        };
        self.0.flags_mut().set_dynamic(Flags::CF, cf);
        if masked == 1 {
            let msb = result & (1 << (bits - 1)) != 0;
            let of = if right {
                msb != (result & (1 << (bits - 2)) != 0)
            } else {
                msb != cf
            };
            self.0.flags_mut().set_dynamic(Flags::OF, of);
        }
        result
    }
    /// Double shifts compute the result from both inputs before changing flags.
    fn double_shift(&mut self, dst: u32, src: u32, count: u8, bits: u32, right: bool) -> u32 {
        let count = count & 31;
        if count == 0 {
            return dst;
        }
        let combined = if right {
            ((src as u64) << bits) | dst as u64
        } else {
            ((dst as u64) << bits) | src as u64
        };
        let result = if right {
            combined >> count
        } else {
            (combined << count) >> bits
        } & width_mask(bits);
        let cf = if right {
            (combined >> (count - 1)) & 1 != 0
        } else {
            (combined >> (2 * bits - count as u32)) & 1 != 0
        };
        let flags = self.0.flags_mut();
        flags.set_dynamic(Flags::CF, cf);
        flags.set_dynamic(Flags::ZF, result == 0);
        flags.set_dynamic(Flags::SF, result & (1 << (bits - 1)) != 0);
        flags.set_dynamic(Flags::PF, (result as u8).count_ones() % 2 == 0);
        if count == 1 {
            flags.set_dynamic(Flags::OF, (dst ^ result as u32) & (1 << (bits - 1)) != 0);
        }
        result as u32
    }
}

impl Alu<'_> {
    pub fn rol8(&mut self, dst: u8, count: u8) -> u8 {
        self.rotate(dst as u32, count, 8, false, false) as u8
    }
    pub fn rol16(&mut self, dst: u16, count: u8) -> u16 {
        self.rotate(dst as u32, count, 16, false, false) as u16
    }
    pub fn rol32(&mut self, dst: u32, count: u8) -> u32 {
        self.rotate(dst as u32, count, 32, false, false) as u32
    }
    pub fn ror8(&mut self, dst: u8, count: u8) -> u8 {
        self.rotate(dst as u32, count, 8, true, false) as u8
    }
    pub fn ror16(&mut self, dst: u16, count: u8) -> u16 {
        self.rotate(dst as u32, count, 16, true, false) as u16
    }
    pub fn ror32(&mut self, dst: u32, count: u8) -> u32 {
        self.rotate(dst as u32, count, 32, true, false) as u32
    }
    pub fn rcl8(&mut self, dst: u8, count: u8) -> u8 {
        self.rotate(dst as u32, count, 8, false, true) as u8
    }
    pub fn rcl16(&mut self, dst: u16, count: u8) -> u16 {
        self.rotate(dst as u32, count, 16, false, true) as u16
    }
    pub fn rcl32(&mut self, dst: u32, count: u8) -> u32 {
        self.rotate(dst as u32, count, 32, false, true) as u32
    }
    pub fn rcr8(&mut self, dst: u8, count: u8) -> u8 {
        self.rotate(dst as u32, count, 8, true, true) as u8
    }
    pub fn rcr16(&mut self, dst: u16, count: u8) -> u16 {
        self.rotate(dst as u32, count, 16, true, true) as u16
    }
    pub fn rcr32(&mut self, dst: u32, count: u8) -> u32 {
        self.rotate(dst as u32, count, 32, true, true) as u32
    }
    pub fn shld16(&mut self, dst: u16, src: u16, count: u8) -> u16 {
        self.double_shift(dst as u32, src as u32, count, 16, false) as u16
    }
    pub fn shld32(&mut self, dst: u32, src: u32, count: u8) -> u32 {
        self.double_shift(dst as u32, src as u32, count, 32, false) as u32
    }
    pub fn shrd16(&mut self, dst: u16, src: u16, count: u8) -> u16 {
        self.double_shift(dst as u32, src as u32, count, 16, true) as u16
    }
    pub fn shrd32(&mut self, dst: u32, src: u32, count: u8) -> u32 {
        self.double_shift(dst as u32, src as u32, count, 32, true) as u32
    }
}
