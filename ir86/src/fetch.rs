//! Fetching instruction

use crate::_prelude_::*;

/// Trait for fetching instruction bytes from the instruction stream.
pub trait Fetch {
    type E;

    /// Fetches an 8-bit value from the instruction stream.
    ///
    /// If returns `Err`, instruction decode will be aborted.
    fn fetch_u8(&mut self) -> Result<u8, Self::E>;

    /// Returns the current instruction pointer
    ///
    /// Branch instructions may use this to calculate target addresses.
    fn current_eip(&self) -> Offset32;

    /// Fetches a signed 8-bit value from the instruction stream
    #[inline]
    fn fetch_i8(&mut self) -> Result<i8, Self::E> {
        self.fetch_u8().map(|b| b as i8)
    }

    /// Fetches a 16-bit value from the instruction stream
    #[inline]
    fn fetch_u16(&mut self) -> Result<u16, Self::E> {
        let low = self.fetch_u8()? as u16;
        let high = self.fetch_u8()? as u16;
        Ok((high << 8) | low)
    }

    /// Fetches a signed 16-bit value from the instruction stream
    #[inline]
    fn fetch_i16(&mut self) -> Result<i16, Self::E> {
        self.fetch_u16().map(|w| w as i16)
    }

    /// Fetches a 32-bit value from the instruction stream
    #[inline]
    fn fetch_u32(&mut self) -> Result<u32, Self::E> {
        let low = self.fetch_u16()? as u32;
        let high = self.fetch_u16()? as u32;
        Ok((high << 16) | low)
    }

    /// Fetches a signed 32-bit value from the instruction stream
    #[inline]
    fn fetch_i32(&mut self) -> Result<i32, Self::E> {
        self.fetch_u32().map(|d| d as i32)
    }
}

/// A simple fetcher that fetches instruction bytes from a byte slice.
///
/// The slice index is used as the instruction pointer, so the slice is
/// treated as an address space starting at 0.
pub struct SimpleFetcher<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> SimpleFetcher<'a> {
    /// Creates a new instance of `SimpleFetcher` with the given slice.
    #[inline]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }
}

impl SimpleFetcher<'_> {
    /// Returns the current position in the instruction stream.
    #[inline]
    pub const fn pos(&self) -> usize {
        self.pos
    }

    /// Sets the current position in the instruction stream.
    #[inline]
    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }
}

impl Fetch for SimpleFetcher<'_> {
    type E = ();

    #[inline]
    fn fetch_u8(&mut self) -> Result<u8, Self::E> {
        if self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            self.pos += 1;
            Ok(b)
        } else {
            Err(())
        }
    }

    #[inline]
    fn fetch_u16(&mut self) -> Result<u16, Self::E> {
        let bytes = self
            .bytes
            .get(self.pos..)
            .and_then(|tail| tail.first_chunk::<2>())
            .ok_or(())?;
        let value = u16::from_le_bytes(*bytes);
        self.pos += 2;
        Ok(value)
    }

    #[inline]
    fn fetch_u32(&mut self) -> Result<u32, Self::E> {
        let bytes = self
            .bytes
            .get(self.pos..)
            .and_then(|tail| tail.first_chunk::<4>())
            .ok_or(())?;
        let value = u32::from_le_bytes(*bytes);
        self.pos += 4;
        Ok(value)
    }

    #[inline]
    fn current_eip(&self) -> Offset32 {
        Offset32(self.pos as u32)
    }
}

/// A fetcher that fetches instruction bytes from a byte slice placed at a base address.
pub struct OffsetFetcher<'a> {
    inner: SimpleFetcher<'a>,
    base: Offset32,
}

impl<'a> OffsetFetcher<'a> {
    /// Creates a new instance of `OffsetFetcher` with the given slice and base address.
    #[inline]
    pub fn new(bytes: &'a [u8], base: Offset32) -> Self {
        Self {
            inner: SimpleFetcher::new(bytes),
            base,
        }
    }
}

impl OffsetFetcher<'_> {
    /// Returns the current position in the instruction stream.
    #[inline]
    pub const fn pos(&self) -> usize {
        self.inner.pos()
    }

    /// Sets the current position in the instruction stream.
    #[inline]
    pub fn set_pos(&mut self, pos: usize) {
        self.inner.set_pos(pos);
    }
}

impl Fetch for OffsetFetcher<'_> {
    type E = ();

    #[inline]
    fn fetch_u8(&mut self) -> Result<u8, Self::E> {
        self.inner.fetch_u8()
    }

    #[inline]
    fn fetch_u16(&mut self) -> Result<u16, Self::E> {
        self.inner.fetch_u16()
    }

    #[inline]
    fn fetch_u32(&mut self) -> Result<u32, Self::E> {
        self.inner.fetch_u32()
    }

    #[inline]
    fn current_eip(&self) -> Offset32 {
        Offset32(self.base.0.wrapping_add(self.inner.pos() as u32))
    }
}
