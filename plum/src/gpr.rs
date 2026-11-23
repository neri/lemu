//! General Purpose Registers

use core::cell::UnsafeCell;
use core::mem::transmute;
use core::sync::atomic::{AtomicU8, AtomicU16, AtomicU32, Ordering};

use crate::prelude::*;

/// General Purpose Register
pub struct GeneralPurposeRegister(UnsafeCell<GprRepr>);

/// Internal Representation of a general purpose register
union GprRepr {
    u32: u32,
    u16: [u16; 2],
    u8: [u8; 4],
}

impl GeneralPurposeRegister {
    #[inline(always)]
    pub const fn new(value: u32) -> Self {
        Self(UnsafeCell::new(GprRepr { u32: value }))
    }

    #[inline(always)]
    unsafe fn repr(&self) -> &GprRepr {
        unsafe { &(*self.0.get()) }
    }

    /// Returns a reference to a 32-bit partial register.
    #[inline(always)]
    pub fn e<'a>(&'a self) -> Gpr32<'a> {
        // Safety: The conversion is safe
        Gpr32(unsafe { transmute(&self.repr().u32) })
    }

    /// Returns a reference to a 16-bit partial register.
    ///
    /// Alias for `x()`
    #[inline(always)]
    pub fn w<'a>(&'a self) -> Gpr16<'a> {
        self.x()
    }

    /// Returns a reference to a 16-bit partial register.
    ///
    /// Alias for `w()`
    #[inline(always)]
    pub fn x<'a>(&'a self) -> Gpr16<'a> {
        // Safety: The conversion is safe
        Gpr16(unsafe { transmute(&self.repr().u16[0]) })
    }

    /// Returns a reference to the lower 8-bit partial register.
    ///
    /// Alias for `l()`
    #[inline(always)]
    pub fn b<'a>(&'a self) -> Gpr8<'a> {
        self.l()
    }

    /// Returns a reference to the lower 8-bit partial register.
    ///
    /// Alias for `b()`
    #[inline(always)]
    pub fn l<'a>(&'a self) -> Gpr8<'a> {
        // Safety: The conversion is safe
        Gpr8(unsafe { transmute(&self.repr().u8[0]) })
    }

    /// Returns a reference to the higher 8-bit partial register.
    #[inline(always)]
    pub fn h<'a>(&'a self) -> Gpr8<'a> {
        // Safety: The conversion is safe
        Gpr8(unsafe { transmute(&self.repr().u8[1]) })
    }
}

impl Clone for GeneralPurposeRegister {
    #[inline]
    fn clone(&self) -> Self {
        GeneralPurposeRegister(UnsafeCell::new(GprRepr {
            u32: self.e().read(),
        }))
    }
}

/// Partial Register
pub trait PartialRegister {
    type ValType: Copy + PartialEq + Eq + PartialOrd + Ord;

    fn read(&self) -> Self::ValType;

    fn write(&self, value: Self::ValType);

    #[inline]
    fn modify<F>(&self, f: F) -> Self::ValType
    where
        F: FnOnce(Self::ValType) -> Self::ValType,
    {
        let mut value = self.read();
        value = f(value);
        self.write(value);
        value
    }
}

/// Partial 32-bit General Purpose Register
pub struct Gpr32<'a>(&'a AtomicU32);

impl PartialRegister for Gpr32<'_> {
    type ValType = u32;

    #[inline]
    fn read(&self) -> Self::ValType {
        self.0.load(Ordering::Relaxed).to_le()
    }

    #[inline]
    fn write(&self, value: Self::ValType) {
        self.0.store(value.to_le(), Ordering::Relaxed);
    }
}

/// Partial 16-bit General Purpose Register
pub struct Gpr16<'a>(&'a AtomicU16);

impl PartialRegister for Gpr16<'_> {
    type ValType = u16;

    #[inline]
    fn read(&self) -> Self::ValType {
        self.0.load(Ordering::Relaxed).to_le()
    }

    #[inline]
    fn write(&self, value: Self::ValType) {
        self.0.store(value.to_le(), Ordering::Relaxed);
    }
}

/// Partial 8-bit General Purpose Register
pub struct Gpr8<'a>(&'a AtomicU8);

impl PartialRegister for Gpr8<'_> {
    type ValType = u8;

    #[inline]
    fn read(&self) -> Self::ValType {
        self.0.load(Ordering::Relaxed)
    }

    #[inline]
    fn write(&self, value: Self::ValType) {
        self.0.store(value, Ordering::Relaxed);
    }
}

/// Segment Register
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentRegister {
    pub sel: SegmentSelector,
    pub base: Linear32,
    pub limit: Limit32,
    pub ar: u16,
}

impl SegmentRegister {
    #[inline]
    pub fn new(sel: SegmentSelector, base: Linear32, limit: Limit32, ar: u16) -> Self {
        Self {
            sel,
            base,
            limit,
            ar,
        }
    }

    /// Loads the segment register with the specified selector and updates the base accordingly.
    #[inline]
    pub fn load_rm(&mut self, sel: SegmentSelector) {
        self.sel = sel;
        self.base = Linear32(sel.0 as u32 * 16);
    }

    /// Initializes the segment register as a data segment with a null selector and default attributes.
    #[inline]
    pub fn init_data_rm(&mut self) {
        self.sel = SegmentSelector(0);
        self.base = Linear32(0);
        self.limit = Limit32(0xffff);
        self.ar = 0x92;
    }

    /// Initializes the segment register as a code segment with a predefined selector and attributes.
    #[inline]
    pub fn init_code_rm(&mut self) {
        self.sel = SegmentSelector(0xf000);
        self.base = Linear32(0xffff_0000);
        self.limit = Limit32(0xffff);
        self.ar = 0x9a;
    }

    #[inline]
    pub fn ea16(&self, offset: Offset16) -> Linear32 {
        Linear32(self.base.0.wrapping_add(offset.0 as u32))
    }

    #[inline]
    pub fn ea32(&self, offset: Offset32) -> Linear32 {
        Linear32(self.base.0.wrapping_add(offset.0))
    }
}
