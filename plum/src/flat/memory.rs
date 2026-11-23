//! Flat guest memory and instruction-boundary invalidation.
//!
//! Cached traces run directly by UPC. A code write patches the current guest
//! instruction's exit with an invalidation Uop, leaving the normal path unchanged.
//! Reverse instruction metadata is consulted at writes/stops, not per dispatch.
use core::ops::Range;

use super::uop::{AddrIndex, Uop};
use super::{Exception, FlatUME};
use crate::_prelude_::*;
use crate::gpr::PartialRegister;

const PAGE_SHIFT: usize = 12;

/// Invalid configuration of a contiguous 32-bit guest address interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitError {
    AddressOverflow,
    EntryOutsideMemory,
    StackOutsideMemory,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Instruction {
    pub source: Offset32,
    pub next: Offset32,
    pub start: AddrIndex,
    pub end: AddrIndex,
}

pub(super) struct Flat {
    pages: Vec<u8>,
    first_page: usize,
    last_page: usize,
    pub eip: Offset32,
    // Kept independently of the cache, including across host edits at a stop.
    pub stopped: Option<(Offset32, Offset32)>,
    pending: bool,
    pub fetch_error: Option<Exception>,
}

impl Flat {
    pub(super) fn new(len: usize, entry: Offset32) -> Self {
        let page_count = len.div_ceil(1 << PAGE_SHIFT);
        Self {
            pages: alloc::vec![0; page_count.div_ceil(8)],
            first_page: usize::MAX,
            last_page: 0,
            eip: entry,
            stopped: None,
            pending: false,
            fetch_error: None,
        }
    }

    fn mark_decoded(&mut self, range: Range<usize>) {
        self.first_page = self.first_page.min(range.start >> PAGE_SHIFT);
        self.last_page = self.last_page.max((range.end - 1) >> PAGE_SHIFT);
        for page in range.start >> PAGE_SHIFT..=(range.end - 1) >> PAGE_SHIFT {
            self.pages[page / 8] |= 1 << (page % 8);
        }
    }

    fn written(&mut self, range: Range<usize>) -> bool {
        if range.is_empty() || self.pending {
            return false;
        }
        // Most data/stack writes are beyond the decoded text pages. Reject
        // these before loading the bitmap; still retain page-level false hits.
        let first = range.start >> PAGE_SHIFT;
        if first > self.last_page {
            return false;
        }
        let last = (range.end - 1) >> PAGE_SHIFT;
        if last < self.first_page {
            return false;
        }
        self.pending = (first.max(self.first_page)..=last.min(self.last_page))
            .any(|p| self.pages[p / 8] & (1 << (p % 8)) != 0);
        self.pending
    }
}

impl FlatUME {
    /// Extend at a host stop without discarding decoded pages or saved control flow.
    pub(super) fn grow_memory(&mut self, len: usize) -> Result<(), ()> {
        let bitmap_len = len.div_ceil(1 << PAGE_SHIFT).div_ceil(8);
        // Reserve both allocations before changing either logical length.
        self.memory
            .try_reserve(len - self.memory.len())
            .map_err(|_| ())?;
        self.flat
            .pages
            .try_reserve(bitmap_len - self.flat.pages.len())
            .map_err(|_| ())?;
        self.memory.resize(len, 0);
        self.flat.pages.resize(bitmap_len, 0);
        Ok(())
    }

    /// Read the entire guest address space.
    #[inline]
    pub fn memory(&self) -> &[u8] {
        &self.memory
    }

    #[inline]
    pub(super) fn memory_range(&self, addr: u32, len: usize) -> Result<Range<usize>, Exception> {
        let fault = Exception::SegmentationViolation(Offset32(addr));
        let base = self.base;
        // The constructor guarantees base + allocation length <= 2^32.
        // For a nonempty access below base, wrapping subtraction therefore
        // produces an offset beyond this allocation. One bounds check covers
        // both ends without a separate underflow branch on every guest load.
        let start = addr.wrapping_sub(base) as usize;
        let limit = self.memory().len().checked_sub(len).ok_or(fault)?;
        if start > limit || (len == 0 && addr < base) {
            return Err(fault);
        }
        Ok(start..start + len)
    }

    /// Write guest bytes atomically with respect to bounds validation.
    /// Cache invalidation occurs at the next safe instruction boundary.
    #[inline]
    pub fn write_bytes(&mut self, address: u32, bytes: &[u8]) -> Result<(), Exception> {
        let range = self.memory_range(address, bytes.len())?;
        self.memory[range.clone()].copy_from_slice(bytes);
        self.notify_write(range);
        Ok(())
    }

    /// Fills the emulated memory at the specified address with the given value for the specified size.
    #[inline]
    pub fn fill_bytes(&mut self, address: u32, value: u8, size: usize) -> Result<(), Exception> {
        let range = self.memory_range(address, size)?;
        self.memory[range.clone()].fill(value);
        self.notify_write(range);
        Ok(())
    }

    /// Call only after validating the entire destination.
    #[cfg(feature = "x87")]
    #[inline]
    pub(super) fn write_range(&mut self, range: Range<usize>, bytes: &[u8]) {
        self.memory[range.clone()].copy_from_slice(bytes);
        self.notify_write(range);
    }

    #[inline]
    pub(super) fn notify_write(&mut self, range: Range<usize>) {
        if !range.is_empty()
            && range.start >> PAGE_SHIFT <= self.flat.last_page
            && (range.end - 1) >> PAGE_SHIFT >= self.flat.first_page
        {
            self.notify_code_write(range);
        }
    }

    #[cold]
    #[inline(never)]
    fn notify_code_write(&mut self, range: Range<usize>) {
        if self.flat.written(range) {
            // The fetched Uop is a copy. Its remaining guest Uops stay intact;
            // the first following Uop is discarded by the flush.
            self.tracer.arm_invalidation();
        }
    }

    fn reset_flat_cache(&mut self) {
        #[cfg(feature = "uop-profile")]
        self.profile.archive(&self.tracer);
        self.tracer.invalidate();
        let flat = &mut self.flat;
        if flat.first_page <= flat.last_page {
            flat.pages[flat.first_page / 8..=flat.last_page / 8].fill(0);
        }
        flat.first_page = usize::MAX;
        flat.last_page = 0;
        flat.pending = false;
    }

    fn flush_pending(&mut self) {
        if self.flat.pending {
            self.reset_flat_cache();
        }
    }

    fn append_trace_exit(&mut self, uop: Uop) -> Result<(), Exception> {
        self.tracer.append_flat(uop).map_err(|error| {
            // Never retain a partially built trace without its boundary slot.
            self.reset_flat_cache();
            error
        })
    }

    pub(super) fn prepare_flat(&mut self) {
        if let Some((source, _)) = self.flat.stopped.take() {
            self.flat.eip = source;
        }
        self.tracer.set_current_upc(AddrIndex::BAD_ADDRESS);
        self.flush_pending();
    }

    #[inline]
    pub(super) fn pending_write(&self) -> bool {
        self.flat.pending
    }

    /// Commit control flow only after this instruction's architectural updates.
    pub(super) fn flat_branch(&mut self, target: Offset32) {
        self.flat.eip = target;
        self.flush_pending();
        let upc = self
            .tracer
            .cached_return(target)
            .or_else(|| {
                let upc = self.tracer.address_map().get(&target).copied()?;
                self.tracer.cache_return(target, upc);
                Some(upc)
            })
            .unwrap_or(AddrIndex::BAD_ADDRESS);
        self.tracer.set_current_upc(upc);
    }

    /// Decode only on a cache miss. Failures beyond completed instructions are
    /// deferred to FetchNext so speculative bytes cannot stop earlier code.
    fn decode_flat_trace(&mut self, eip: Offset32) -> Result<AddrIndex, Exception> {
        let mut first = None;
        let mut next = eip;
        for _ in 0..128 {
            if let Some(&upc) = self.tracer.address_map().get(&next) {
                if let Some(first) = first {
                    self.append_trace_exit(Uop::Jump(upc))?;
                    #[cfg(feature = "flag-elision")]
                    self.tracer.finish_elision();
                    return Ok(first);
                }
                return Ok(upc);
            }
            let base = self.base;
            let instruction = match self.tracer.decode_flat(&self.memory, next) {
                Ok(instruction) => instruction,
                Err(error) => {
                    if first.is_none() {
                        return Err(error);
                    }
                    break;
                }
            };
            let emitted =
                &self.tracer.uop_cache()[instruction.start.0 as usize..instruction.end.0 as usize];
            let stop = emitted.iter().any(Uop::ends_trace);
            let start = instruction.source.0.wrapping_sub(base) as usize;
            let len = instruction.next.0.wrapping_sub(instruction.source.0) as usize;
            self.flat.mark_decoded(start..start + len);
            self.tracer.record_instruction(instruction);
            first.get_or_insert(instruction.start);
            next = instruction.next;
            if stop {
                break;
            }
        }
        self.append_trace_exit(Uop::FetchNext(next))?;
        #[cfg(feature = "flag-elision")]
        self.tracer.finish_elision();
        Ok(first.expect("nonempty trace or an earlier decode error"))
    }

    pub(super) fn resolve_flat_target(&mut self, eip: Offset32) -> Result<AddrIndex, Exception> {
        if let Some(&upc) = self.tracer.address_map().get(&eip) {
            return Ok(upc);
        }
        match self.decode_flat_trace(eip) {
            Ok(upc) => Ok(upc),
            Err(error) => {
                // An instruction-fetch failure belongs to the requested EIP,
                // not to a previous guest branch or a synthetic continuation.
                self.flat.eip = eip;
                self.tracer.set_current_upc(AddrIndex::BAD_ADDRESS);
                Err(error)
            }
        }
    }

    /// Keep cached fetch scalar and inline so Wasm can extract the Uop payload
    /// without materializing an Option<Uop> on every dispatch.
    #[inline(always)]
    pub(super) fn fetch_cached(&mut self) -> Option<Uop> {
        match self.tracer.cached_uop() {
            Some(Uop::FetchNext(eip)) => self.fetch_flat_slow(Some(eip)),
            Some(uop) => Some(uop),
            None => self.fetch_flat_slow(None),
        }
    }

    #[inline(always)]
    fn fetch_flat_slow(&mut self, continuation: Option<Offset32>) -> Option<Uop> {
        let result = match continuation {
            Some(eip) => self.continue_flat(eip),
            None => self.enter_flat(),
        };
        match result {
            Ok(()) => self.tracer.cached_uop(),
            Err(error) => {
                self.flat.fetch_error = Some(error);
                None
            }
        }
    }

    pub(super) fn continue_flat(&mut self, eip: Offset32) -> Result<(), Exception> {
        let old = self.tracer.current_upc();
        let upc = self.resolve_flat_target(eip)?;
        self.tracer.set_current_upc(old);
        self.tracer.replace(Uop::Jump(upc));
        self.tracer.set_current_upc(upc);
        Ok(())
    }

    pub(super) fn enter_flat(&mut self) -> Result<(), Exception> {
        let eip = self.flat.eip;
        let upc = self.resolve_flat_target(eip)?;
        self.tracer.cache_return(eip, upc);
        self.tracer.set_current_upc(upc);
        Ok(())
    }

    pub(super) fn stop_flat(&mut self) {
        let instruction = self.tracer.instruction_at(self.tracer.current_upc());
        let flat = &mut self.flat;
        let (source, next) = instruction.map_or((flat.eip, flat.eip), |i| (i.source, i.next));
        flat.stopped = Some((source, next));
        flat.eip = source;
        self.state.eip().write(source.0);
        self.state.flags_mut().resolve();
        self.flush_pending();
    }
}
