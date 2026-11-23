use crate::prelude::*;

/// 16bit Far Pointer Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Far16Ptr(pub u32);

impl Far16Ptr {
    /// Packs an offset and selector into a 16:16 far pointer.
    #[inline]
    pub const fn new(offset: Offset16, selector: SegmentSelector) -> Self {
        Self(((selector.0 as u32) << 16) | offset.0 as u32)
    }

    #[inline]
    pub const fn sel(&self) -> SegmentSelector {
        SegmentSelector((self.0 >> 16) as u16)
    }

    #[inline]
    pub const fn offset(&self) -> Offset16 {
        Offset16(self.0 as u16)
    }

    /// Returns the linear address
    #[inline]
    pub const fn as_linear(&self) -> Linear32 {
        let sel = self.sel().0 as u32;
        let offset = self.offset().0 as u32;
        Linear32(sel * 16 + offset)
    }
}

/// 32bit Far Pointer Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Far32Ptr {
    pub offset: Offset32,
    pub selector: SegmentSelector,
}

impl Far32Ptr {
    #[inline]
    pub const fn new(offset: Offset32, selector: SegmentSelector) -> Self {
        Self { offset, selector }
    }

    #[inline]
    pub const fn sel(&self) -> SegmentSelector {
        self.selector
    }

    #[inline]
    pub const fn offset(&self) -> Offset32 {
        self.offset
    }
}

/// 32bit Linear Address Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Linear32(pub u32);

impl Linear32 {
    pub const MIN: Self = Linear32(0);

    pub const MAX: Self = Linear32(u32::MAX);

    #[inline]
    pub const fn wrapping_add(self, rhs: u32) -> Self {
        Linear32(self.0.wrapping_add(rhs))
    }
}

/// Segment Limit Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limit32(pub u32);

impl Limit32 {
    pub const MAX: Self = Limit32(u32::MAX);
}

/// 16-bit Segment Limit Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limit16(pub u16);

impl Limit16 {
    pub const MAX: Self = Limit16(u16::MAX);
}

/// I/O Port Address Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IoPort(pub u16);

/// 32-bit physical byte address (after paging).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Physical32(pub u32);

impl Physical32 {
    pub const MIN: Self = Physical32(0);

    pub const MAX: Self = Physical32(u32::MAX);
}
