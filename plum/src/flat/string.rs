//! Validated ordinary-memory spans. Kernels never change guest flags or indices.
use ir86::types::Offset32;

use super::uop::{StringKind, StringOp, StringRepeat, StringWidth, Uop};
use super::{Exception, FlatUME};
use crate::gpr::PartialRegister;
use crate::string_ops::{self, CompareRepeat, compare, load, range, span};

impl FlatUME {
    // Keep the large width/direction kernels out of the hot Uop dispatcher.
    // Inlining them slows arithmetic-only guests on Wasm (see CPU benchmark).
    #[inline(never)]
    pub(super) fn execute_string(&mut self, op: StringOp) -> Result<(), Exception> {
        let kind = op.kind();
        let width = op.width();
        let repeat = op.repeat();
        let addr32 = op.is_addr32();
        let comparison = matches!(kind, StringKind::Cmps | StringKind::Scas);
        let valid = match repeat {
            StringRepeat::Once => true,
            StringRepeat::Count => !comparison,
            StringRepeat::WhileZero | StringRepeat::WhileNotZero => comparison,
        };
        if !addr32 || !valid {
            return Err(Exception::Unimplemented2(Uop::String(op)));
        }
        // Select width and direction once, before entering any element kernel.
        match (width, self.state.flags().df()) {
            (StringWidth::Byte, false) => self.string_spans::<1, false>(kind, repeat),
            (StringWidth::Word, false) => self.string_spans::<2, false>(kind, repeat),
            (StringWidth::Dword, false) => self.string_spans::<4, false>(kind, repeat),
            (StringWidth::Byte, true) => self.string_spans::<1, true>(kind, repeat),
            (StringWidth::Word, true) => self.string_spans::<2, true>(kind, repeat),
            (StringWidth::Dword, true) => self.string_spans::<4, true>(kind, repeat),
        }
    }

    fn string_spans<const N: usize, const REVERSE: bool>(
        &mut self,
        kind: StringKind,
        repeat: StringRepeat,
    ) -> Result<(), Exception> {
        let repeated = repeat != StringRepeat::Once;
        let mut remaining = if repeated { self.state.ecx().read() } else { 1 };
        let use_src = matches!(kind, StringKind::Movs | StringKind::Lods | StringKind::Cmps);
        let use_dest = kind != StringKind::Lods;
        let shift = N.trailing_zeros();
        let mask = u32::MAX >> ((4 - N) * 8);
        let accumulator = self.state.eax().read() & mask;
        while remaining != 0 {
            let esi = self.state.esi().read();
            let edi = self.state.edi().read();
            let base = self.base;
            let src_offset = esi.checked_sub(base);
            let dest_offset = edi.checked_sub(base);
            let mut count = remaining;
            // Preserve single-element fault ordering: source before destination.
            if use_src {
                let len = self.memory().len();
                count = src_offset.map_or(0, |src| span(len, src, shift, REVERSE, count));
                if count == 0 {
                    return Err(Exception::SegmentationViolation(Offset32(esi)));
                }
            }
            if use_dest {
                count = dest_offset.map_or(0, |dest| {
                    span(self.memory().len(), dest, shift, REVERSE, count)
                });
                if count == 0 {
                    return Err(Exception::SegmentationViolation(Offset32(edi)));
                }
            }
            let src_offset = src_offset.unwrap_or(0);
            let dest_offset = dest_offset.unwrap_or(0);
            let mut consumed = count;
            let mut last = None;
            match kind {
                StringKind::Movs => self.move_span::<N, REVERSE>(src_offset, dest_offset, count),
                StringKind::Stos => {
                    let dest = &mut &mut self.memory[range::<N, REVERSE>(dest_offset, count)];
                    string_ops::fill::<N>(dest, accumulator);
                }
                StringKind::Lods => {
                    let src = self.memory();
                    let bytes = &src[range::<N, REVERSE>(src_offset, count)];
                    let index = if REVERSE { 0 } else { bytes.len() - N };
                    let value = load::<N>(&bytes[index..index + N]);
                    match N {
                        1 => self.state.al().write(value as u8),
                        2 => self.state.ax().write(value as u16),
                        4 => self.state.eax().write(value),
                        _ => unreachable!(),
                    }
                }
                StringKind::Cmps | StringKind::Scas => {
                    // Borrow the data region only once: source and destination
                    // may overlap. Neither comparison slice is mutated.
                    let data = self.memory();
                    let source = if kind == StringKind::Cmps {
                        let src = data;
                        Some(&src[range::<N, REVERSE>(src_offset, count)])
                    } else {
                        None
                    };
                    let dest = &data[range::<N, REVERSE>(dest_offset, count)];
                    let (done, a, b) = compare::<N, REVERSE>(
                        source,
                        dest,
                        accumulator,
                        match repeat {
                            StringRepeat::Once => CompareRepeat::Once,
                            StringRepeat::WhileZero => CompareRepeat::WhileZero,
                            StringRepeat::WhileNotZero => CompareRepeat::WhileNotZero,
                            StringRepeat::Count => unreachable!(),
                        },
                    );
                    consumed = done;
                    last = Some((a, b));
                }
            }
            if matches!(kind, StringKind::Movs | StringKind::Stos) {
                self.notify_write(range::<N, REVERSE>(dest_offset, count));
            }
            #[cfg(feature = "uop-profile")]
            {
                self.profile.counters.string_elements += consumed as u64;
                if repeated {
                    self.profile.counters.rep_elements += consumed as u64;
                }
            }
            let distance = consumed.wrapping_mul(N as u32);
            let delta = if REVERSE {
                distance.wrapping_neg()
            } else {
                distance
            };
            if use_src {
                self.state.esi().write(esi.wrapping_add(delta));
            }
            if use_dest {
                self.state.edi().write(edi.wrapping_add(delta));
            }
            remaining -= consumed;
            if repeated {
                self.state.ecx().write(remaining);
            }
            if let Some((a, b)) = last {
                let stop = match repeat {
                    StringRepeat::Once => true,
                    StringRepeat::WhileZero => a != b,
                    StringRepeat::WhileNotZero => a == b,
                    StringRepeat::Count => unreachable!(),
                };
                if stop || remaining == 0 {
                    match N {
                        1 => {
                            self.alu().sub8(a as u8, b as u8);
                        }
                        2 => {
                            self.alu().sub16(a as u16, b as u16);
                        }
                        4 => {
                            self.alu().sub32(a, b);
                        }
                        _ => unreachable!(),
                    }
                    return Ok(());
                }
                // No flags changed while scanning. A fault in the next span
                // therefore leaves the instruction's entry flags intact.
            }
        }
        Ok(())
    }

    fn move_span<const N: usize, const REVERSE: bool>(&mut self, esi: u32, edi: u32, count: u32) {
        let src_range = range::<N, REVERSE>(esi, count);
        let dest_range = range::<N, REVERSE>(edi, count);
        let data = &mut self.memory;
        string_ops::copy::<N, REVERSE>(data, src_range, dest_range, count);
    }
}
