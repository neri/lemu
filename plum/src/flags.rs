//! The FLAGS register

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

use crate::cpu::Generation;
mod lazy;
use ir86::encoding::CC;
pub use lazy::LazyOp;

/// x86 FLAGS register representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags(u32);

impl Flags {
    /// Carry flag
    pub const CF: Self = Self(0x0000_0001);
    /// Reserved Always 1
    // pub const _VF: Self = Self(0x0000_0002);
    /// Parity flag
    pub const PF: Self = Self(0x0000_0004);
    /// Adjust flag
    pub const AF: Self = Self(0x0000_0010);
    /// Zero flag
    pub const ZF: Self = Self(0x0000_0040);
    /// Sign flag
    pub const SF: Self = Self(0x0000_0080);
    /// Trap flag
    pub const TF: Self = Self(0x0000_0100);
    /// Interrupt enable flag
    pub const IF: Self = Self(0x0000_0200);
    /// Direction flag
    pub const DF: Self = Self(0x0000_0400);
    /// Overflow flag
    pub const OF: Self = Self(0x0000_0800);
    /// I/O privilege level
    pub const IOPL3: Self = Self(0x0000_3000);
    /// Nested task flag
    pub const NT: Self = Self(0x0000_4000);
    /// Mode flag (NEC V30)
    pub const MD: Self = Self(0x0000_8000);
    /// Resume flag
    pub const RF: Self = Self(0x0001_0000);
    /// Virtual 8086 mode flag
    pub const VM: Self = Self(0x0002_0000);
    /// Alignment check
    pub const AC: Self = Self(0x0004_0000);
    /// Virtual interrupt flag
    pub const VIF: Self = Self(0x0008_0000);
    /// Virtual interrupt pending
    pub const VIP: Self = Self(0x0010_0000);
    /// Able to use CPUID instruction
    pub const ID: Self = Self(0x0020_0000);

    /// All flags cleared (used for initialization)
    pub const ZERO: Self = Self(0);

    /// All flags set (used for testing)
    pub const NOT_ZERO: Self = Self(!0);

    #[inline]
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// Returns the raw bits of the flags.
    #[inline]
    pub const fn bits(&self) -> u32 {
        self.0
    }

    #[inline]
    pub fn contains(&self, value: Self) -> bool {
        (self.0 & value.0) == value.0
    }
}

impl BitAnd<Self> for Flags {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign<Self> for Flags {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr<Self> for Flags {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign<Self> for Flags {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor<Self> for Flags {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl BitXorAssign<Self> for Flags {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Not for Flags {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

#[derive(Clone, Copy)]
pub struct FlagsRegister {
    inner: FlagSet,
    lazy_op: LazyOp,
}

#[derive(Clone, Copy)]
pub struct FlagSet {
    dynamic_value: Flags,
    static_value: Flags,
    valid_mask: Flags,
    always_0_mask: Flags,
    always_1_mask: Flags,
}

impl FlagsRegister {
    /// Creates a new `FlagsRegister` struct with the appropriate masks based on the CPU generation.
    pub fn new(generation: Generation) -> Self {
        let mask0 = Self::mask_0_for(generation);
        let mask1 = Self::mask_1_for(generation);
        Self {
            inner: FlagSet {
                dynamic_value: Flags::ZERO,
                static_value: Flags::ZERO,
                valid_mask: Flags::NOT_ZERO,
                always_0_mask: Flags::from_bits(mask0),
                always_1_mask: Flags::from_bits(mask1),
            },
            lazy_op: LazyOp::default(),
        }
    }

    #[inline]
    pub const fn mask_0_for(generation: Generation) -> u32 {
        let base = 0x003f_7fd5;
        match generation {
            // Generation::I8086 => base & 0x0000_0fff,
            // Generation::I186 => base & 0x0000_0fff,
            // Generation::I286 => base & 0x0000_7fff,
            Generation::I386 => base & 0x0003_ffff,
            Generation::I486 => base & 0x0027_ffff,
            Generation::Pentium | Generation::P6 | Generation::P7 | Generation::NetBurst => base,
        }
    }

    #[inline]
    pub const fn mask_1_for(generation: Generation) -> u32 {
        let base = 0x0000_0002;
        match generation {
            // Generation::I8086 => base | 0x0000_f000
            // Generation::I186 => base | 0x0000_f000,
            // Generation::I286 => base,
            Generation::I386
            | Generation::I486
            | Generation::Pentium
            | Generation::P6
            | Generation::P7
            | Generation::NetBurst => base,
        }
    }

    #[inline(always)]
    pub fn df(&self) -> bool {
        self.inner.df()
    }

    /// EFLAGS.VM, without resolving the lazy arithmetic flags.
    #[inline(always)]
    pub fn vm(&self) -> bool {
        self.inner.vm()
    }

    #[inline(always)]
    pub fn zf(&self) -> bool {
        self.inner.zf()
    }

    #[inline(always)]
    pub fn set_bits(&mut self, bits: Flags) {
        self.inner.set_bits(bits);
    }

    #[inline(always)]
    pub(super) fn set_lazy_op(&mut self, lazy_op: LazyOp) {
        self.lazy_op = lazy_op;
    }

    /// Clears dynamic flags and sets the ZF flag according to the result.
    #[inline]
    pub(crate) fn reset_flags_with_zf(&mut self, is_zero: bool) {
        self.inner.dynamic_value = if is_zero { Flags::ZF } else { Flags::ZERO };
        self.inner.valid_mask = Flags::ZF;
    }

    /// Adjusts the flags after INC and DEC operations.
    ///
    /// Note: INC and DEC do not affect CF; `cf` is the carry before the operation.
    #[inline]
    pub(crate) fn adjust_after_inc_dec(&mut self, cf: bool, is_zero: bool) {
        let cf = if cf { Flags::CF } else { Flags::ZERO };
        let zf = if is_zero { Flags::ZF } else { Flags::ZERO };
        self.inner.dynamic_value = cf | zf;
        self.inner.valid_mask = Flags::ZF | Flags::CF;
    }

    /// Adjusts the flags after logical operations (AND, OR, XOR).
    ///
    /// Note: Clear OF and CF on after logical operations, and set ZF according to the result. AF is undefined for logical operations.
    #[inline]
    pub(crate) fn adjust_after_logic_op(&mut self, is_zero: bool) {
        let mask: Flags = Flags::OF | Flags::ZF | Flags::AF | Flags::CF;
        let zf = if is_zero { Flags::ZF } else { Flags::ZERO };
        self.inner.dynamic_value = (self.inner.dynamic_value & !mask) | zf;
        self.inner.valid_mask = mask;
    }

    /// Adjusts the flags after a multiplication operation
    #[inline]
    pub(crate) fn adjust_after_mul(&mut self) {
        // the CF and OF flags are set depending on the result of the multiplication, other dynamic flags are undefined, so we clear them
        self.inner.dynamic_value = Flags::ZERO;
        self.inner.valid_mask = Flags::ZERO;
    }

    /// Floating-point compares overwrite every arithmetic flag. Discard the
    /// previous lazy operation without evaluating flags that will be replaced.
    #[cfg(feature = "x87")]
    #[inline]
    pub(crate) fn set_compare_result(&mut self, cf: bool, zf: bool, pf: bool) {
        self.inner.dynamic_value =
            Flags::from_bits(u32::from(cf) | (u32::from(pf) << 2) | (u32::from(zf) << 6));
        self.inner.valid_mask = FlagSet::DYNAMIC_VALUE_MASK;
    }

    /// Resolves lazy flags and returns the final value of the FLAGS register.
    #[inline]
    pub fn resolve(&mut self) -> Flags {
        self.lazy_op.resolve_all_flags(&mut self.inner);
        self.inner.adjust_after_resolve()
    }

    #[inline]
    pub fn recompute_cf(&mut self) -> bool {
        self.lazy_op.recompute_cf(&mut self.inner)
    }

    #[inline]
    pub fn resolve_cf(&mut self) -> bool {
        self.lazy_op.resolve_cf(&mut self.inner)
    }

    #[inline]
    pub fn complement_cf(&mut self) -> bool {
        let cf = self.resolve_cf();
        let new_cf = !cf;
        self.inner.set_dynamic(Flags::CF, new_cf);
        new_cf
    }

    #[inline]
    pub fn resolve_sf(&mut self) -> bool {
        self.lazy_op.resolve_sf(&mut self.inner)
    }

    #[inline]
    pub fn resolve_of(&mut self) -> bool {
        self.lazy_op.resolve_of(&mut self.inner)
    }

    #[inline]
    pub fn resolve_pf(&mut self) -> bool {
        self.lazy_op.resolve_pf(&mut self.inner)
    }

    /// Signed less-than is SF xor OF. If both still come from SUB/CMP,
    /// comparing the original operands also handles subtraction overflow.
    fn signed_less(&mut self) -> bool {
        if (self.inner.valid_mask & (Flags::SF | Flags::OF)) == Flags::ZERO {
            match self.lazy_op {
                LazyOp::Sub8(a, b) => return (a as i8) < b as i8,
                LazyOp::Sub16(a, b) => return (a as i16) < b as i16,
                LazyOp::Sub32(a, b) => return (a as i32) < b as i32,
                _ => {}
            }
        }
        self.resolve_sf() != self.resolve_of()
    }

    /// Evaluates the condition code based on the current flags.
    pub fn eval_cc(&mut self, cc: CC) -> bool {
        match cc {
            CC::O => self.resolve_of(),
            CC::NO => !self.resolve_of(),
            CC::C => self.resolve_cf(),
            CC::NC => !self.resolve_cf(),
            CC::Z => self.inner.zf(),
            CC::NZ => !self.inner.zf(),
            CC::BE => self.inner.zf() || self.resolve_cf(),
            CC::NBE => !(self.inner.zf() || self.resolve_cf()),
            CC::S => self.resolve_sf(),
            CC::NS => !self.resolve_sf(),
            CC::P => self.resolve_pf(),
            CC::NP => !self.resolve_pf(),
            CC::L => self.signed_less(),
            CC::NL => !self.signed_less(),
            CC::LE => self.inner.zf() || self.signed_less(),
            CC::NLE => !(self.inner.zf() || self.signed_less()),
        }
    }

    /// Sets the dynamic value of the FLAGS register and marks all flags as valid.
    #[inline]
    pub fn set(&mut self, value: Flags) {
        self.inner.set(value);
    }

    /// Clears the dynamic value of the FLAGS register and marks all flags as valid.
    #[inline]
    pub fn clear(&mut self, value: Flags) {
        self.inner.clear(value);
    }

    /// Sets the dynamic value of a specific flag in the FLAGS register.
    #[inline]
    pub fn set_dynamic(&mut self, flag: Flags, value: bool) {
        self.inner.set_dynamic(flag, value);
    }

    /// Sets the static value of a specific flag in the FLAGS register.
    #[inline]
    pub fn set_static(&mut self, flag: Flags, value: bool) {
        self.inner.set_static(flag, value);
    }
}

impl FlagSet {
    pub const DYNAMIC_VALUE_MASK: Flags = Flags::from_bits(0x0000_08d5);

    #[inline]
    pub fn set_bits(&mut self, bits: Flags) {
        self.dynamic_value = bits & Self::DYNAMIC_VALUE_MASK;
        self.static_value = bits & !Self::DYNAMIC_VALUE_MASK;
        self.valid_mask = Self::DYNAMIC_VALUE_MASK;
    }

    #[inline]
    fn adjust_after_resolve(&mut self) -> Flags {
        self.dynamic_value = self.dynamic_value & FlagSet::DYNAMIC_VALUE_MASK;
        self.static_value = self.static_value & !FlagSet::DYNAMIC_VALUE_MASK;
        (self.dynamic_value | self.static_value) & self.always_0_mask | self.always_1_mask
    }

    /// Sets the dynamic value of the FLAGS register and marks all flags as valid.
    #[inline]
    pub fn set(&mut self, value: Flags) {
        debug_assert!(value.bits() & !Self::DYNAMIC_VALUE_MASK.bits() == 0);
        self.dynamic_value |= value;
        self.valid_mask |= value;
    }

    /// Clears the dynamic value of the FLAGS register and marks all flags as valid.
    #[inline]
    pub fn clear(&mut self, value: Flags) {
        debug_assert!(value.bits() & !Self::DYNAMIC_VALUE_MASK.bits() == 0);
        self.dynamic_value &= !value;
        self.valid_mask |= value;
    }

    /// Sets the dynamic value of a specific flag in the FLAGS register.
    #[inline]
    pub fn set_dynamic(&mut self, flag: Flags, value: bool) {
        debug_assert!(flag.bits() & !Self::DYNAMIC_VALUE_MASK.bits() == 0);
        if value {
            self.set(flag);
        } else {
            self.clear(flag);
        }
    }

    /// Sets the static value of a specific flag in the FLAGS register.
    #[inline]
    pub fn set_static(&mut self, flag: Flags, value: bool) {
        debug_assert!(flag.bits() & Self::DYNAMIC_VALUE_MASK.bits() == 0);
        if value {
            self.static_value |= flag;
        } else {
            self.static_value &= !flag;
        }
    }

    #[inline]
    pub fn df(&self) -> bool {
        self.static_value.contains(Flags::DF)
    }

    /// EFLAGS.VM, without resolving the lazy arithmetic flags.
    #[inline]
    pub fn vm(&self) -> bool {
        self.static_value.contains(Flags::VM)
    }

    #[inline]
    pub fn zf(&self) -> bool {
        self.dynamic_value.contains(Flags::ZF)
    }

    /// # Safety
    /// The caller must ensure that the SF flag is valid before calling this function.
    /// # Safety
    unsafe fn sf(&self) -> bool {
        self.dynamic_value.contains(Flags::SF)
    }

    /// # Safety
    /// The caller must ensure that the OF flag is valid before calling this function.
    #[inline]
    unsafe fn of(&self) -> bool {
        self.dynamic_value.contains(Flags::OF)
    }

    /// # Safety
    /// The caller must ensure that the PF flag is valid before calling this function.
    #[inline]
    unsafe fn pf(&self) -> bool {
        self.dynamic_value.contains(Flags::PF)
    }
}
