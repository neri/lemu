//! Trace based micro IR decoder

use alloc::collections::btree_map::Entry;

use crate::_prelude_::*;
use crate::cpu::{RtReg, RtReg8};
use crate::harvard::Exception;
use crate::harvard::uop::{
    AddrIndex, BinaryRmwKind, BitAction, FuncIndex, LoadExtension, LoopKind, ShiftRmwOp,
    StringKind, StringOp, StringRepeat, StringWidth, TodoKind, UnaryRmwOp, UnsupportedKind, Uop,
    UopMinor,
};

/// Register indices move across words; immediate indices stay in one word.
#[derive(Clone, Copy)]
enum BitIndex {
    Register(RtReg),
    Immediate(u8),
}

// A fixed direct-mapped cache: 128 bytes, with no allocation per return.
const RETURN_CACHE_ENTRIES: usize = 16;

/// Trace based micro IR decoder
pub struct TraceDecoder {
    #[cfg(feature = "uop-profile")]
    pub(super) profile_starts: BTreeMap<u32, (u32, String)>,
    code: Box<[u8]>,
    eip_to_fetch: Offset32,
    fetch_size: usize,
    max_step: usize,
    #[cfg(feature = "flag-elision-analysis")]
    effects: Vec<(u32, crate::elision::Effect)>,

    return_targets: [(Offset32, AddrIndex); RETURN_CACHE_ENTRIES],
    current_upc: AddrIndex,
    uop_cache: Vec<Uop>,
    address_map: BTreeMap<Offset32, AddrIndex>,

    functions: Vec<(Offset32, AddrIndex)>,
    function_map: BTreeMap<Offset32, FuncIndex>,
}

impl TraceDecoder {
    /// Creates a new instance
    #[inline]
    pub fn new(code: Box<[u8]>, eip: Offset32) -> Self {
        let trailing_zeros = code.iter().rev().take_while(|&&b| b == 0).count();
        let fetch_size = code.len() - trailing_zeros;

        let mut result = Self {
            #[cfg(feature = "uop-profile")]
            profile_starts: BTreeMap::new(),
            code,
            eip_to_fetch: eip,
            fetch_size,
            max_step: 1024,
            #[cfg(feature = "flag-elision-analysis")]
            effects: Vec::new(),

            return_targets: [(Offset32(0), AddrIndex::BAD_ADDRESS); RETURN_CACHE_ENTRIES],
            current_upc: AddrIndex(0),
            uop_cache: Vec::new(),
            address_map: BTreeMap::new(),

            function_map: BTreeMap::new(),
            functions: Vec::new(),
        };
        result._fetch_and_decode(1);
        result
    }

    #[inline]
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// Returns the current instruction and fetches if necessary
    #[inline]
    pub fn fetch_uop(&mut self) -> Option<Uop> {
        if let Some(&uop) = self.uop_cache.get(self.current_upc.0 as usize) {
            match uop {
                Uop::FetchNext(eip) => {
                    return self.handle_fetch_next(eip);
                }
                _ => return Some(uop),
            }
        } else {
            self._fetch_uop2()
        }
    }

    // Keep the scalar fetch ABI when both independent engines are compiled.
    // Otherwise LLVM may spill Option<Uop> on every cached dispatch in Wasm.
    fn handle_fetch_next(&mut self, eip: Offset32) -> Option<Uop> {
        self.eip_to_fetch = eip;
        if self.uop_cache.len() == 1 + self.current_upc.0 as usize {
            self.uop_cache.pop();
        } else {
            let addr_index = AddrIndex(self.uop_cache.len() as u32);
            self.replace(Uop::Jump(addr_index));
            self.current_upc = addr_index;
        }
        self.address_map.remove(&eip);
        self.fetch_and_decode();
        self.uop_cache.get(self.current_upc.0 as usize).copied()
    }

    fn _fetch_uop2(&mut self) -> Option<Uop> {
        self.fetch_and_decode();
        self.uop_cache.get(self.current_upc.0 as usize).copied()
    }

    /// Advances the current UPC to the next instruction in the cache.
    #[inline]
    pub fn advance_upc(&mut self) {
        self.current_upc.0 += 1;
    }

    /// Returns the current instruction pointer.
    #[inline]
    pub fn current_upc(&self) -> AddrIndex {
        self.current_upc
    }

    /// Sets the current instruction pointer.
    #[inline]
    pub fn set_current_upc(&mut self, upc: AddrIndex) {
        self.current_upc = upc;
    }

    /// Returns the address map.
    ///
    /// # Note
    /// This function is intended for testing and debugging purposes. It may be removed or modified in future versions.
    #[inline]
    pub fn address_map(&self) -> &BTreeMap<Offset32, AddrIndex> {
        &self.address_map
    }

    /// Returns the function map.
    ///
    /// # Note
    /// This function is intended for testing and debugging purposes. It may be removed or modified in future versions.
    #[inline]
    pub fn functions(&self) -> &[(Offset32, AddrIndex)] {
        &self.functions
    }

    /// Returns the uop cache.
    ///
    /// # Note
    /// This function is intended for testing and debugging purposes. It may be removed or modified in future versions.
    #[inline]
    pub fn uop_cache(&self) -> &[Uop] {
        &self.uop_cache
    }

    /// Resolves the target address to an AddrIndex, fetching and decoding if necessary.
    #[inline]
    pub fn resolve_target(&mut self, target: Offset32) -> Option<AddrIndex> {
        if let Some(&addr_index) = self.address_map.get(&target) {
            Some(addr_index)
        } else {
            self._resolve_target2(target)
        }
    }

    fn _resolve_target2(&mut self, target: Offset32) -> Option<AddrIndex> {
        let old_upc = self.current_upc;
        self.eip_to_fetch = target;
        self.fetch_and_decode();
        self.current_upc = old_upc;
        self.address_map.get(&target).copied()
    }

    /// Sets the instruction pointer to the specified EIP and updates the current UPC accordingly.
    #[inline]
    pub fn set_eip(&mut self, eip: Offset32) -> Result<(), Exception> {
        self.eip_to_fetch = eip;
        self.current_upc = self
            .resolve_target(eip)
            .ok_or(Exception::SegmentationViolation(eip))?;
        Ok(())
    }

    /// Resolve the actual return address popped from the guest stack.
    /// Code is immutable and cached UPCs remain executable as traces grow:
    /// FetchNext is replaced by its decoded entry or a Jump to that entry.
    /// Code replacement or UPC compaction would require invalidating this cache.
    #[inline]
    pub(super) fn set_return_eip(&mut self, eip: Offset32) -> Result<(), Exception> {
        let slot = (eip.0 as usize) & (RETURN_CACHE_ENTRIES - 1);
        let (tag, upc) = self.return_targets[slot];
        if tag == eip && upc != AddrIndex::BAD_ADDRESS {
            self.eip_to_fetch = eip;
            self.current_upc = upc;
            Ok(())
        } else {
            self.resolve_return_miss(eip, slot)
        }
    }

    #[inline(never)]
    fn resolve_return_miss(&mut self, eip: Offset32, slot: usize) -> Result<(), Exception> {
        self.set_eip(eip)?;
        self.return_targets[slot] = (eip, self.current_upc);
        Ok(())
    }

    /// Replaces the current instruction with a new Uop in the cache.
    #[inline]
    pub fn replace(&mut self, new_uop: Uop) {
        self.uop_cache[self.current_upc.0 as usize] = new_uop;
    }

    /// Resolves the function at the given index and updates the current UPC to point to it.
    #[inline]
    pub fn resolve_and_invoke_function(&mut self, func_index: FuncIndex) {
        let function = self.functions[func_index.0 as usize];
        if function.1 != AddrIndex::BAD_ADDRESS {
            // Function already resolved
            self.current_upc = function.1;
        } else {
            self._resolve_and_invoke_function2(func_index, function);
        }
    }

    fn _resolve_and_invoke_function2(
        &mut self,
        func_index: FuncIndex,
        function: (Offset32, AddrIndex),
    ) {
        if let Some(&target) = self.address_map.get(&function.0) {
            // Function is not yet resolved, but we have a target address in the address map
            self.functions[func_index.0 as usize].1 = target;
            self.current_upc = target;
        } else {
            // Function is not yet resolved, and we don't have a target address in the address map
            self.set_eip(function.0).unwrap();
            if let Some(&target) = self.address_map.get(&function.0) {
                self.functions[func_index.0 as usize].1 = target;
                // self.current_upc = target;
            } else {
                todo!("Failed to resolve function at EIP={:#010x}", function.0.0);
            }
        }
    }

    /// Generates a best effort LEA sequence for the given memory operand and destination register.
    #[inline]
    pub fn generate_lea(rd: RtReg, ma: MemOpr32) -> Uop {
        match ma.base_index {
            BaseIndex32::DispOnly => Uop::LoadConst(rd, ma.disp.0),
            BaseIndex32::Base(base) => {
                if ma.disp.0 == 0 {
                    Uop::Move(rd, base.into())
                } else {
                    Uop::LeaBD(rd, base.into(), ma.disp)
                }
            }
            BaseIndex32::Sib(sib) => Uop::LeaSIB(rd, sib, ma.disp),
        }
    }

    /// MMX/SSE/SSE2: the shared x87-table Uop; 16-bit addressing is a Todo.
    #[cfg(feature = "sse")]
    fn lower_simd(uop_cache: &mut Vec<Uop>, ir: IrOp) {
        use crate::fpu::uop::FpuMinorUop;
        let op = FpuMinorUop::simd_lowering(ir, |ma| match ma {
            MemOpr::A32(ma) => Some((Self::emit_lea_opt(uop_cache, ma), ma.segment)),
            MemOpr::A16(_) => None,
        });
        uop_cache.push(match op {
            Some(op) => Uop::Fpu(op),
            None => Uop::Minor(UopMinor::Todo(TodoKind::SimdAddress16)),
        });
    }

    /// Emits a best effort LEA sequence for the given memory operand and returns the destination register.
    pub fn emit_lea_opt(uop_cache: &mut Vec<Uop>, ma: MemOpr32) -> RtReg {
        // Default destination register for LEA is RtReg::MemAddr, which is a special register used for memory addressing.
        let rd = RtReg::MemAddr;
        match ma.base_index {
            BaseIndex32::DispOnly => {
                uop_cache.push(Uop::LoadConst(rd, ma.disp.0));
            }
            BaseIndex32::Base(base) => {
                if ma.disp.0 == 0 {
                    return base.into();
                } else {
                    uop_cache.push(Uop::LeaBD(rd, base.into(), ma.disp));
                }
            }
            BaseIndex32::Sib(sib) => {
                uop_cache.push(Uop::LeaSIB(rd, sib, ma.disp));
            }
        }
        rd
    }

    /// Emits a best effort load sequence for the given memory operand and destination register.
    pub fn emit_load32(uop_cache: &mut Vec<Uop>, rd: RtReg, ma: MemOpr32) {
        match ma.segment {
            SrIndex::CS => {
                let mem_addr = Self::emit_lea_opt(uop_cache, ma);
                uop_cache.push(Uop::LoadR32CS(rd, mem_addr));
            }
            _ => match ma.base_index {
                BaseIndex32::DispOnly => uop_cache.push(Uop::LoadBD32(rd, RtReg::Zero, ma.disp)),
                BaseIndex32::Base(base) => {
                    if ma.disp.0 == 0 {
                        uop_cache.push(Uop::LoadR32(rd, base.into()));
                    } else {
                        uop_cache.push(Uop::LoadBD32(rd, base.into(), ma.disp));
                    }
                }
                BaseIndex32::Sib(sib) => uop_cache.push(Uop::LoadSIB32(rd, sib, ma.disp)),
            },
        }
    }

    /// Emits a best effort load sequence for the given memory operand and destination register (8-bit).
    pub fn emit_load8(uop_cache: &mut Vec<Uop>, rd: RtReg8, ma: MemOpr32) {
        match ma.segment {
            SrIndex::CS => {
                let mem_addr = Self::emit_lea_opt(uop_cache, ma);
                uop_cache.push(Uop::LoadR8CS(rd, mem_addr));
            }
            _ => match ma.base_index {
                BaseIndex32::DispOnly => {
                    uop_cache.push(Uop::LoadConst(RtReg::MemAddr, ma.disp.0));
                    uop_cache.push(Uop::LoadR8(rd, RtReg::MemAddr));
                }
                BaseIndex32::Base(base) => {
                    if ma.disp.0 == 0 {
                        uop_cache.push(Uop::LoadR8(rd, base.into()));
                    } else {
                        let addr = Self::emit_lea_opt(uop_cache, ma);
                        uop_cache.push(Uop::LoadR8(rd, addr));
                    }
                }
                BaseIndex32::Sib(_sib) => {
                    let addr = Self::emit_lea_opt(uop_cache, ma);
                    uop_cache.push(Uop::LoadR8(rd, addr));
                }
            },
        }
    }

    /// Emits a best effort load sequence for the given memory operand and destination register (16-bit).
    pub fn emit_load16(uop_cache: &mut Vec<Uop>, rd: RtReg, ma: MemOpr32) {
        match ma.segment {
            SrIndex::CS => {
                let mem_addr = Self::emit_lea_opt(uop_cache, ma);
                uop_cache.push(Uop::LoadR16CS(rd, mem_addr));
            }
            _ => match ma.base_index {
                BaseIndex32::DispOnly => {
                    uop_cache.push(Uop::LoadConst(RtReg::MemAddr, ma.disp.0));
                    uop_cache.push(Uop::LoadR16(rd, RtReg::MemAddr));
                }
                BaseIndex32::Base(base) => {
                    if ma.disp.0 == 0 {
                        uop_cache.push(Uop::LoadR16(rd, base.into()));
                    } else {
                        let addr = Self::emit_lea_opt(uop_cache, ma);
                        uop_cache.push(Uop::LoadR16(rd, addr));
                    }
                }
                BaseIndex32::Sib(_sib) => {
                    let addr = Self::emit_lea_opt(uop_cache, ma);
                    uop_cache.push(Uop::LoadR16(rd, addr));
                }
            },
        }
    }

    /// Generates a best effort store sequence for the given memory operand and source register.
    pub fn generate_store32(rd: RtReg, ma: MemOpr32) -> Uop {
        match ma.segment {
            SrIndex::CS => Uop::Minor(UopMinor::WriteCodeFault(rd)),
            _ => match ma.base_index {
                BaseIndex32::DispOnly => Uop::StoreBD32(rd, RtReg::Zero, ma.disp),
                BaseIndex32::Base(base) => {
                    if ma.disp.0 == 0 {
                        Uop::StoreR32(rd, base.into())
                    } else {
                        Uop::StoreBD32(rd, base.into(), ma.disp)
                    }
                }
                BaseIndex32::Sib(sib) => Uop::StoreSIB32(rd, sib, ma.disp),
            },
        }
    }

    /// Select the effective address once and reject code writes before any update.
    /// The factory runs while decoding, never in the execution dispatcher.
    fn emit_memory_update<F>(uops: &mut Vec<Uop>, ma: MemOpr32, make_uop: F)
    where
        F: FnOnce(RtReg) -> Uop,
    {
        let addr = Self::emit_lea_opt(uops, ma);
        uops.push(if ma.segment == SrIndex::CS {
            Uop::Minor(UopMinor::WriteCodeFault(addr))
        } else {
            make_uop(addr)
        });
    }

    /// Low-frequency updates and measured shift exceptions share the existing
    /// load/ALU/store path. Capture EA once and reject CS before changing flags.
    fn emit_memory_fallback(uops: &mut Vec<Uop>, ma: MemOpr32, op: Uop, width: u8) {
        let addr = Self::emit_lea_opt(uops, ma);
        if ma.segment == SrIndex::CS {
            uops.push(Uop::Minor(UopMinor::WriteCodeFault(addr)));
            return;
        }
        uops.push(match width {
            8 => Uop::LoadR8(RtReg8::MemData, addr),
            16 => Uop::LoadR16(RtReg::MemData, addr),
            32 => Uop::LoadR32(RtReg::MemData, addr),
            _ => unreachable!(),
        });
        uops.push(op);
        uops.push(match width {
            8 => Uop::StoreR8(RtReg8::MemData, addr),
            16 => Uop::StoreR16(RtReg::MemData, addr),
            32 => Uop::StoreR32(RtReg::MemData, addr),
            _ => unreachable!(),
        });
    }

    /// Updates fuse index adjustment and memory access. Read-only BT and CS faults
    /// retain the adjusted address in MemAddr for their existing multi-Uop path.
    fn emit_bit_memory(
        uops: &mut Vec<Uop>,
        ma: MemOpr32,
        action: BitAction,
        narrow: bool,
        index: BitIndex,
    ) {
        if action != BitAction::Test && ma.segment != SrIndex::CS {
            Self::emit_memory_update(uops, ma, |addr| match (narrow, index) {
                (true, BitIndex::Immediate(index)) => Uop::BitIRMW16(action, addr, index),
                (false, BitIndex::Immediate(index)) => Uop::BitIRMW(action, addr, index),
                (true, BitIndex::Register(index)) => Uop::BitRRMW16(action, addr, index),
                (false, BitIndex::Register(index)) => Uop::BitRRMW(action, addr, index),
            });
            return;
        }
        let addr = RtReg::MemAddr;
        let data = RtReg::MemData;
        uops.push(Self::generate_lea(addr, ma));
        if let BitIndex::Register(reg) = index {
            uops.push(if narrow {
                Uop::BitAddr16(reg)
            } else {
                Uop::BitAddr(reg)
            });
        }
        if ma.segment == SrIndex::CS && action != BitAction::Test {
            uops.push(Uop::Minor(UopMinor::WriteCodeFault(addr)));
            return;
        }
        uops.push(match (narrow, ma.segment == SrIndex::CS) {
            (true, true) => Uop::LoadR16CS(data, addr),
            (false, true) => Uop::LoadR32CS(data, addr),
            (true, false) => Uop::LoadR16(data, addr),
            (false, false) => Uop::LoadR32(data, addr),
        });
        uops.push(match (narrow, index) {
            (true, BitIndex::Register(reg)) => Uop::BitR16(action, data, reg),
            (false, BitIndex::Register(reg)) => Uop::BitR(action, data, reg),
            (true, BitIndex::Immediate(imm)) => Uop::BitI16(action, data, imm),
            (false, BitIndex::Immediate(imm)) => Uop::BitI(action, data, imm),
        });
    }

    /// Returns a Uop that represents a conditional jump based on the provided condition code and target address index.
    #[inline]
    pub fn jcc_opt(cc: CC, target: AddrIndex) -> Uop {
        match cc {
            CC::Z => Uop::Jz(target),
            CC::NZ => Uop::Jnz(target),
            _ => Uop::Jcc(cc, target),
        }
    }

    /// Fetches and decodes next chunk of instructions.
    #[inline]
    pub fn fetch_and_decode(&mut self) {
        self._fetch_and_decode(self.max_step);
    }

    /// Fetches and decodes next chunk of instructions, up to `max_step` instructions.
    fn _fetch_and_decode(&mut self, mut max_step: usize) {
        let current_eip = self.eip_to_fetch;
        let mut decoder = Decoder::use32();
        let mut fetcher = SimpleFetcher::new(&self.code);
        fetcher.set_pos(current_eip.0 as usize);

        #[cfg(feature = "flag-elision-analysis")]
        let mut effects = core::mem::take(&mut self.effects);
        while max_step > 0 && fetcher.pos() < self.fetch_size {
            max_step -= 1;
            let source_eip = fetcher.current_eip();

            let entry = match self.address_map.entry(source_eip) {
                Entry::Vacant(entry) => entry,
                Entry::Occupied(entry) => {
                    if source_eip != current_eip {
                        self.uop_cache.push(Uop::Jump(*entry.get()));
                    }
                    break;
                }
            };

            let Ok(ir) = decoder.decode(&mut fetcher) else {
                break;
            };

            let addr_index = AddrIndex(self.uop_cache.len() as u32);
            entry.insert(addr_index);
            #[cfg(feature = "flag-elision-analysis")]
            effects.push((addr_index.0, crate::elision::effect(ir)));
            #[cfg(feature = "uop-profile")]
            self.profile_starts
                .insert(addr_index.0, (source_eip.0, alloc::format!("{ir:?}")));

            match ir {
                IrOp::ESC_Rb(opcode, reg, index) => {
                    #[cfg(feature = "x87")]
                    self.uop_cache.push(Uop::Fpu(
                        crate::fpu::uop::FpuMinorUop::register(opcode.opcode, reg, index as u8)
                            .at(source_eip.0),
                    ));
                    #[cfg(not(feature = "x87"))]
                    {
                        let _ = (opcode, reg, index);
                        self.uop_cache
                            .push(Uop::Minor(UopMinor::Todo(TodoKind::FpuDisabled)));
                    }
                }
                IrOp::ESC_Mb(opcode, reg, MemOpr::A32(ma)) => {
                    #[cfg(feature = "x87")]
                    {
                        let address = Self::emit_lea_opt(&mut self.uop_cache, ma);
                        self.uop_cache.push(Uop::Fpu(
                            crate::fpu::uop::FpuMinorUop::memory(
                                opcode.opcode,
                                reg,
                                address,
                                ma.segment,
                                opcode.is_opsize32,
                            )
                            .at(source_eip.0),
                        ));
                    }
                    #[cfg(not(feature = "x87"))]
                    {
                        let _ = (opcode, reg, ma);
                        self.uop_cache
                            .push(Uop::Minor(UopMinor::Todo(TodoKind::FpuDisabled)));
                    }
                }
                IrOp::ESC_Mb(_, _, MemOpr::A16(_)) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::Todo(TodoKind::FpuAddress16)));
                }
                // MMX/SSE/SSE2: #UD without `sse`; otherwise the shared x87
                // Uop, which stops unimplemented forms at run time.
                IrOp::SIMD(..) | IrOp::SIMD_R(..) | IrOp::SIMD_M(..) | IrOp::SIMD_S(..) => {
                    #[cfg(feature = "sse")]
                    Self::lower_simd(&mut self.uop_cache, ir);
                    #[cfg(not(feature = "sse"))]
                    self.uop_cache.push(Uop::Minor(UopMinor::InvalidOpcode));
                }
                IrOp::WAIT => {
                    #[cfg(feature = "x87")]
                    self.uop_cache
                        .push(Uop::Fpu(crate::fpu::uop::FpuMinorUop::new(
                            crate::fpu::uop::FpuOp::Wait,
                            0,
                        )));
                    #[cfg(not(feature = "x87"))]
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::Todo(TodoKind::FpuDisabled)));
                }
                IrOp::AAA => self.uop_cache.push(Uop::Minor(UopMinor::Aaa)),
                IrOp::AAD_Ib(base) => self.uop_cache.push(Uop::Minor(UopMinor::Aad(base))),
                IrOp::AAM_Ib(base) => self.uop_cache.push(Uop::Minor(UopMinor::Aam(base))),
                IrOp::AAS => self.uop_cache.push(Uop::Minor(UopMinor::Aas)),
                IrOp::ADC_Mb_Ib(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW8(BinaryRmwKind::Adc, addr, imm)
                    });
                }
                IrOp::ADC_Mb_Rb(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW8(BinaryRmwKind::Adc, addr, rs.into())
                    });
                }
                IrOp::ADC_Md_Id(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW(BinaryRmwKind::Adc, addr, imm)
                    });
                }
                IrOp::ADC_Md_Rd(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW(BinaryRmwKind::Adc, addr, rs.into())
                    });
                }
                IrOp::ADC_Mw_Iw(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW16(BinaryRmwKind::Adc, addr, imm)
                    });
                }
                IrOp::ADC_Mw_Rw(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW16(BinaryRmwKind::Adc, addr, rs.upgrade().into())
                    });
                }
                IrOp::ADC_Rb_Ib(rd, imm) => {
                    self.uop_cache.push(Uop::AdcI8(rd.into(), imm));
                }
                IrOp::ADC_Rb_Mb(rd, MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::AdcR8(rd.into(), RtReg8::MemData));
                }
                IrOp::ADC_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::AdcR8(rd.into(), rs.into()));
                }
                IrOp::ADC_Rd_Id(rd, imm) => {
                    self.uop_cache.push(Uop::AdcI(rd.into(), imm));
                }
                IrOp::ADC_Rd_Md(rd, MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::AdcR(rd.into(), RtReg::MemData));
                }
                IrOp::ADC_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::AdcR(rd.into(), rs.into()));
                }
                IrOp::ADC_Rw_Iw(rd, imm) => {
                    self.uop_cache.push(Uop::AdcI16(rd.upgrade().into(), imm));
                }
                IrOp::ADC_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::AdcR16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::ADC_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::AdcR16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::ADD_Mb_Ib(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW8(BinaryRmwKind::Add, addr, imm)
                    });
                }
                IrOp::ADD_Mb_Rb(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::AddRMW8(addr, rs.into())
                    });
                }
                IrOp::ADD_Md_Id(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW(BinaryRmwKind::Add, addr, imm)
                    });
                }
                IrOp::ADD_Md_Rd(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::AddRMW(addr, rs.into())
                    });
                }
                IrOp::ADD_Mw_Iw(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW16(BinaryRmwKind::Add, addr, imm)
                    });
                }
                IrOp::ADD_Mw_Rw(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW16(BinaryRmwKind::Add, addr, rs.upgrade().into())
                    });
                }
                IrOp::ADD_Rb_Ib(rd, imm) => {
                    self.uop_cache.push(Uop::AddI8(rd.into(), imm));
                }
                IrOp::ADD_Rb_Mb(rd, MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::AddR8(rd.into(), RtReg8::MemData));
                }

                IrOp::ADD_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::AddR8(rd.into(), rs.into()));
                }
                IrOp::ADD_Rd_Id(rd, imm) => {
                    self.uop_cache.push(Uop::AddI(rd.into(), imm));
                }
                IrOp::ADD_Rd_Md(rd, MemOpr::A32(ma)) => {
                    let addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    self.uop_cache
                        .push(Uop::AddLoad(rd.into(), addr, ma.segment));
                }
                IrOp::ADD_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::AddR(rd.into(), rs.into()));
                }
                IrOp::ADD_Rw_Iw(rd, imm) => {
                    self.uop_cache.push(Uop::AddI16(rd.upgrade().into(), imm));
                }
                IrOp::ADD_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::AddR16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::ADD_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::AddR16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::AND_Mb_Ib(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW8(BinaryRmwKind::And, addr, imm)
                    });
                }
                IrOp::AND_Mb_Rb(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW8(BinaryRmwKind::And, addr, rs.into())
                    });
                }
                IrOp::AND_Md_Id(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW(BinaryRmwKind::And, addr, imm)
                    });
                }
                IrOp::AND_Md_Rd(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::AndRMW(addr, rs.into())
                    });
                }
                IrOp::AND_Mw_Iw(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW16(BinaryRmwKind::And, addr, imm)
                    });
                }
                IrOp::AND_Mw_Rw(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW16(BinaryRmwKind::And, addr, rs.upgrade().into())
                    });
                }
                IrOp::AND_Rb_Ib(rd, imm) => {
                    self.uop_cache.push(Uop::AndI8(rd.into(), imm));
                }
                IrOp::AND_Rb_Mb(rd, MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::AndR8(rd.into(), RtReg8::MemData));
                }
                IrOp::AND_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::AndR8(rd.into(), rs.into()));
                }
                IrOp::AND_Rd_Id(rd, imm) => {
                    self.uop_cache.push(Uop::AndI(rd.into(), imm));
                }
                IrOp::AND_Rd_Md(rd, MemOpr::A32(ma)) => {
                    let addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    self.uop_cache
                        .push(Uop::AndLoad(rd.into(), addr, ma.segment));
                }
                IrOp::AND_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::AndR(rd.into(), rs.into()));
                }
                IrOp::AND_Rw_Iw(rd, imm) => {
                    self.uop_cache.push(Uop::AndI16(rd.upgrade().into(), imm));
                }
                IrOp::AND_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::AndR16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::AND_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::AndR16(rd.upgrade().into(), rs.upgrade().into()));
                }

                IrOp::BSF_Rd_Md(rd, MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::BsfR(rd.into(), RtReg::MemData));
                }
                IrOp::BSF_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::BsfR(rd.into(), rs.into()));
                }
                IrOp::BSF_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::BsfR16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::BSF_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::BsfR16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::BSR_Rd_Md(rd, MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::BsrR(rd.into(), RtReg::MemData));
                }
                IrOp::BSR_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::BsrR(rd.into(), rs.into()));
                }
                IrOp::BSR_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::BsrR16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::BSR_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::BsrR16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::BSWAP_Rd(rd) => {
                    self.uop_cache.push(Uop::BswapR(rd.into()));
                }
                IrOp::BTC_Md_Ib(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Complement,
                        false,
                        BitIndex::Immediate(index),
                    );
                }
                IrOp::BTC_Md_Rd(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Complement,
                        false,
                        BitIndex::Register(index.into()),
                    );
                }
                IrOp::BTC_Mw_Ib(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Complement,
                        true,
                        BitIndex::Immediate(index),
                    );
                }
                IrOp::BTC_Mw_Rw(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Complement,
                        true,
                        BitIndex::Register(index.upgrade().into()),
                    );
                }
                IrOp::BTC_Rd_Ib(rd, index) => {
                    self.uop_cache
                        .push(Uop::BitI(BitAction::Complement, rd.into(), index));
                }
                IrOp::BTC_Rd_Rd(rd, index) => {
                    self.uop_cache
                        .push(Uop::BitR(BitAction::Complement, rd.into(), index.into()));
                }
                IrOp::BTC_Rw_Ib(rd, index) => {
                    self.uop_cache.push(Uop::BitI16(
                        BitAction::Complement,
                        rd.upgrade().into(),
                        index,
                    ));
                }
                IrOp::BTC_Rw_Rw(rd, index) => {
                    self.uop_cache.push(Uop::BitR16(
                        BitAction::Complement,
                        rd.upgrade().into(),
                        index.upgrade().into(),
                    ));
                }
                IrOp::BTR_Md_Ib(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Reset,
                        false,
                        BitIndex::Immediate(index),
                    );
                }
                IrOp::BTR_Md_Rd(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Reset,
                        false,
                        BitIndex::Register(index.into()),
                    );
                }
                IrOp::BTR_Mw_Ib(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Reset,
                        true,
                        BitIndex::Immediate(index),
                    );
                }
                IrOp::BTR_Mw_Rw(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Reset,
                        true,
                        BitIndex::Register(index.upgrade().into()),
                    );
                }
                IrOp::BTR_Rd_Ib(rd, index) => {
                    self.uop_cache
                        .push(Uop::BitI(BitAction::Reset, rd.into(), index));
                }
                IrOp::BTR_Rd_Rd(rd, index) => {
                    self.uop_cache
                        .push(Uop::BitR(BitAction::Reset, rd.into(), index.into()));
                }
                IrOp::BTR_Rw_Ib(rd, index) => {
                    self.uop_cache
                        .push(Uop::BitI16(BitAction::Reset, rd.upgrade().into(), index));
                }
                IrOp::BTR_Rw_Rw(rd, index) => {
                    self.uop_cache.push(Uop::BitR16(
                        BitAction::Reset,
                        rd.upgrade().into(),
                        index.upgrade().into(),
                    ));
                }
                IrOp::BTS_Md_Ib(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Set,
                        false,
                        BitIndex::Immediate(index),
                    );
                }
                IrOp::BTS_Md_Rd(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Set,
                        false,
                        BitIndex::Register(index.into()),
                    );
                }
                IrOp::BTS_Mw_Ib(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Set,
                        true,
                        BitIndex::Immediate(index),
                    );
                }
                IrOp::BTS_Mw_Rw(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Set,
                        true,
                        BitIndex::Register(index.upgrade().into()),
                    );
                }
                IrOp::BTS_Rd_Ib(rd, index) => {
                    self.uop_cache
                        .push(Uop::BitI(BitAction::Set, rd.into(), index));
                }
                IrOp::BTS_Rd_Rd(rd, index) => {
                    self.uop_cache
                        .push(Uop::BitR(BitAction::Set, rd.into(), index.into()));
                }
                IrOp::BTS_Rw_Ib(rd, index) => {
                    self.uop_cache
                        .push(Uop::BitI16(BitAction::Set, rd.upgrade().into(), index));
                }
                IrOp::BTS_Rw_Rw(rd, index) => {
                    self.uop_cache.push(Uop::BitR16(
                        BitAction::Set,
                        rd.upgrade().into(),
                        index.upgrade().into(),
                    ));
                }
                IrOp::BT_Md_Ib(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Test,
                        false,
                        BitIndex::Immediate(index),
                    );
                }
                IrOp::BT_Md_Rd(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Test,
                        false,
                        BitIndex::Register(index.into()),
                    );
                }
                IrOp::BT_Mw_Ib(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Test,
                        true,
                        BitIndex::Immediate(index),
                    );
                }
                IrOp::BT_Mw_Rw(MemOpr::A32(ma), index) => {
                    Self::emit_bit_memory(
                        &mut self.uop_cache,
                        ma,
                        BitAction::Test,
                        true,
                        BitIndex::Register(index.upgrade().into()),
                    );
                }
                IrOp::BT_Rd_Ib(rd, index) => {
                    self.uop_cache
                        .push(Uop::BitI(BitAction::Test, rd.into(), index));
                }
                IrOp::BT_Rd_Rd(rd, index) => {
                    self.uop_cache
                        .push(Uop::BitR(BitAction::Test, rd.into(), index.into()));
                }
                IrOp::BT_Rw_Ib(rd, index) => {
                    self.uop_cache
                        .push(Uop::BitI16(BitAction::Test, rd.upgrade().into(), index));
                }
                IrOp::BT_Rw_Rw(rd, index) => {
                    self.uop_cache.push(Uop::BitR16(
                        BitAction::Test,
                        rd.upgrade().into(),
                        index.upgrade().into(),
                    ));
                }
                IrOp::CALL_Jd(target) => {
                    let func_index = self.function_map.get(&target).copied().unwrap_or_else(|| {
                        let func_index = FuncIndex(self.functions.len() as u16);
                        let target2 = self
                            .address_map
                            .get(&target)
                            .copied()
                            .unwrap_or(AddrIndex::BAD_ADDRESS);
                        self.functions.push((target, target2));
                        self.function_map.insert(target, func_index);
                        func_index
                    });
                    self.uop_cache
                        .push(Uop::Call(func_index, fetcher.current_eip()));
                    break;
                }
                IrOp::CALL_Md(MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::CallR(RtReg::MemData, fetcher.current_eip()));
                    break;
                }
                IrOp::CALL_Rd(rd) => {
                    self.uop_cache
                        .push(Uop::CallR(rd.into(), fetcher.current_eip()));
                    break;
                }

                IrOp::CDQ => {
                    self.uop_cache.push(Uop::Cdq);
                }
                IrOp::CLC => self.uop_cache.push(Uop::Minor(UopMinor::Clc)),
                IrOp::CLD => {
                    self.uop_cache.push(Uop::Minor(UopMinor::Cld));
                }
                IrOp::CMC => self.uop_cache.push(Uop::Minor(UopMinor::Cmc)),
                IrOp::CMOV_Rd_Md(cc, rd, MemOpr::A32(ma)) => {
                    // CMOV reads memory even when the condition is false.
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::Cmov(cc, rd.into(), RtReg::MemData));
                }
                IrOp::CMOV_Rd_Rd(cc, rd, rs) => {
                    self.uop_cache.push(Uop::Cmov(cc, rd.into(), rs.into()));
                }
                IrOp::CMOV_Rw_Mw(cc, rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::Cmov16(cc, rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::CMOV_Rw_Rw(cc, rd, rs) => {
                    self.uop_cache
                        .push(Uop::Cmov16(cc, rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::CMPSB(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Cmps,
                        StringWidth::Byte,
                        StringRepeat::Once,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::CMPSD(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Cmps,
                        StringWidth::Dword,
                        StringRepeat::Once,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::CMPSW(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Cmps,
                        StringWidth::Word,
                        StringRepeat::Once,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::CMP_Mb_Ib(MemOpr::A32(ma), imm) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::CmpI8(RtReg8::MemData, imm));
                }
                IrOp::CMP_Mb_Rb(MemOpr::A32(ma), rs) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::CmpR8(RtReg8::MemData, rs.into()));
                }
                IrOp::CMP_Md_Id(MemOpr::A32(ma), imm) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::CmpI(RtReg::MemData, imm));
                }
                IrOp::CMP_Md_Rd(MemOpr::A32(ma), rs) => {
                    let addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    self.uop_cache
                        .push(Uop::CmpMemReg(addr, rs.into(), ma.segment));
                }
                IrOp::CMP_Mw_Iw(MemOpr::A32(ma), imm) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::CmpI16(RtReg::MemData, imm));
                }
                IrOp::CMP_Mw_Rw(MemOpr::A32(ma), rs) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::CmpR16(RtReg::MemData, rs.upgrade().into()));
                }
                IrOp::CMP_Rb_Ib(rd, imm) => {
                    self.uop_cache.push(Uop::CmpI8(rd.into(), imm));
                }
                IrOp::CMP_Rb_Mb(rd, MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::CmpR8(rd.into(), RtReg8::MemData));
                }
                IrOp::CMP_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::CmpR8(rd.into(), rs.into()));
                }
                IrOp::CMP_Rd_Id(rd, imm) => {
                    self.uop_cache.push(Uop::CmpI(rd.into(), imm));
                }
                IrOp::CMP_Rd_Md(rd, MemOpr::A32(ma)) => {
                    let addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    self.uop_cache
                        .push(Uop::CmpRegMem(rd.into(), addr, ma.segment));
                }
                IrOp::CMP_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::CmpR(rd.into(), rs.into()));
                }
                IrOp::CMP_Rw_Iw(rd, imm) => {
                    self.uop_cache.push(Uop::CmpI16(rd.upgrade().into(), imm));
                }
                IrOp::CMP_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::CmpR16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::CMP_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::CmpR16(rd.upgrade().into(), rs.upgrade().into()));
                }

                IrOp::CMPXCHG_Mb_Rb(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::CmpxchgRMW8(addr, rs.into())
                    });
                }
                IrOp::CMPXCHG_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::CmpxchgR8(rd.into(), rs.into()));
                }
                IrOp::CMPXCHG_Mw_Rw(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::CmpxchgRMW16(addr, rs.upgrade().into())
                    });
                }
                IrOp::CMPXCHG_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::CmpxchgR16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::CMPXCHG_Md_Rd(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::CmpxchgRMW(addr, rs.into())
                    });
                }
                IrOp::CMPXCHG_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::CmpxchgR(rd.into(), rs.into()));
                }
                IrOp::CPUID => {
                    self.uop_cache.push(Uop::Minor(UopMinor::Cpuid));
                }
                IrOp::CWD => self.uop_cache.push(Uop::Cwd),
                IrOp::DAA => self.uop_cache.push(Uop::Minor(UopMinor::Daa)),
                IrOp::DAS => self.uop_cache.push(Uop::Minor(UopMinor::Das)),
                IrOp::DEC_Mb(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::UnaryRMW(UnaryRmwOp::Dec8, addr)
                    });
                }
                IrOp::DEC_Md(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::UnaryRMW(UnaryRmwOp::Dec, addr)
                    });
                }
                IrOp::DEC_Mw(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::UnaryRMW(UnaryRmwOp::Dec16, addr)
                    });
                }
                IrOp::DEC_Rb(rd) => {
                    self.uop_cache.push(Uop::DecR8(rd.into()));
                }
                IrOp::DEC_Rd(rd) => {
                    self.uop_cache.push(Uop::DecR(rd.into()));
                }
                IrOp::DEC_Rw(rd) => {
                    self.uop_cache.push(Uop::DecR16(rd.upgrade().into()));
                }
                IrOp::DIV_Mb(MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::DivR8(RtReg8::MemData));
                }
                IrOp::DIV_Md(MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::DivR(RtReg::MemData));
                }
                IrOp::DIV_Mw(MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::DivR16(RtReg::MemData));
                }
                IrOp::DIV_Rb(rs) => {
                    self.uop_cache.push(Uop::DivR8(rs.into()));
                }
                IrOp::DIV_Rd(rs) => {
                    self.uop_cache.push(Uop::DivR(rs.into()));
                }
                IrOp::DIV_Rw(rs) => {
                    self.uop_cache.push(Uop::DivR16(rs.upgrade().into()));
                }
                IrOp::ENTER_Iw_Ib(bytes, level) => {
                    // Nesting levels >= 1 are deliberately unsupported.
                    self.uop_cache.push(if level & 31 == 0 {
                        Uop::Minor(UopMinor::Enter(bytes))
                    } else {
                        Uop::Minor(UopMinor::Unsupported(UnsupportedKind::NestedEnter))
                    });
                    if level & 31 != 0 {
                        break;
                    }
                }
                IrOp::IDIV_Mb(MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::IDivR8(RtReg8::MemData));
                }
                IrOp::IDIV_Md(MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::IDivR(RtReg::MemData));
                }
                IrOp::IDIV_Mw(MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::IDivR16(RtReg::MemData));
                }
                IrOp::IDIV_Rb(rs) => {
                    self.uop_cache.push(Uop::IDivR8(rs.into()));
                }
                IrOp::IDIV_Rd(rs) => {
                    self.uop_cache.push(Uop::IDivR(rs.into()));
                }
                IrOp::IDIV_Rw(rs) => {
                    self.uop_cache.push(Uop::IDivR16(rs.upgrade().into()));
                }
                IrOp::IMUL_Mb(MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::IMulFullR8(RtReg8::MemData));
                }
                IrOp::IMUL_Md(MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::IMulFullR(RtReg::MemData));
                }
                IrOp::IMUL_Mw(MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::IMulFullR16(RtReg::MemData));
                }
                IrOp::IMUL_Rb(rs) => {
                    self.uop_cache.push(Uop::IMulFullR8(rs.into()));
                }
                IrOp::IMUL_Rd(rs) => {
                    self.uop_cache.push(Uop::IMulFullR(rs.into()));
                }
                IrOp::IMUL_Rd_Md(rd, MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::IMulR(rd.into(), RtReg::MemData));
                }
                IrOp::IMUL_Rd_Md_Id(rd, MemOpr::A32(ma), imm) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::IMulRI(rd.into(), RtReg::MemData, imm as i32));
                }
                IrOp::IMUL_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::IMulR(rd.into(), rs.into()));
                }
                IrOp::IMUL_Rd_Rd_Id(rd, rs, imm) => {
                    self.uop_cache
                        .push(Uop::IMulRI(rd.into(), rs.into(), imm as i32));
                }
                IrOp::IMUL_Rw(rs) => {
                    self.uop_cache.push(Uop::IMulFullR16(rs.upgrade().into()));
                }
                IrOp::IMUL_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::IMulR16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::IMUL_Rw_Mw_Iw(rd, MemOpr::A32(ma), imm) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::IMulRI16(
                        rd.upgrade().into(),
                        RtReg::MemData,
                        imm as i16,
                    ));
                }
                IrOp::IMUL_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::IMulR16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::IMUL_Rw_Rw_Iw(rd, rs, imm) => {
                    self.uop_cache.push(Uop::IMulRI16(
                        rd.upgrade().into(),
                        rs.upgrade().into(),
                        imm as i16,
                    ));
                }
                IrOp::INC_Mb(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::UnaryRMW(UnaryRmwOp::Inc8, addr)
                    });
                }
                IrOp::INC_Md(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::UnaryRMW(UnaryRmwOp::Inc, addr)
                    });
                }
                IrOp::INC_Mw(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::UnaryRMW(UnaryRmwOp::Inc16, addr)
                    });
                }
                IrOp::INC_Rb(rd) => {
                    self.uop_cache.push(Uop::IncR8(rd.into()));
                }
                IrOp::INC_Rd(rd) => {
                    self.uop_cache.push(Uop::IncR(rd.into()));
                }
                IrOp::INC_Rw(rd) => {
                    self.uop_cache.push(Uop::IncR16(rd.upgrade().into()));
                }
                IrOp::INSB(_)
                | IrOp::INSD(_)
                | IrOp::INSW(_)
                | IrOp::IN_Ad_Dx
                | IrOp::IN_Ad_Ib(_)
                | IrOp::IN_Al_Dx
                | IrOp::IN_Al_Ib(_)
                | IrOp::IN_Aw_Dx
                | IrOp::IN_Aw_Ib(_)
                | IrOp::OUTSB(_)
                | IrOp::OUTSD(_)
                | IrOp::OUTSW(_)
                | IrOp::OUT_Dx_Ad
                | IrOp::OUT_Dx_Al
                | IrOp::OUT_Dx_Aw
                | IrOp::OUT_Ib_Ad(_)
                | IrOp::OUT_Ib_Al(_)
                | IrOp::OUT_Ib_Aw(_)
                | IrOp::REP_INSB(_)
                | IrOp::REP_INSD(_)
                | IrOp::REP_INSW(_)
                | IrOp::REP_OUTSB(_)
                | IrOp::REP_OUTSD(_)
                | IrOp::REP_OUTSW(_) => {
                    self.uop_cache.push(Uop::Minor(UopMinor::PortIo));
                    break;
                }
                IrOp::INT_Ib(ib) => {
                    self.uop_cache.push(Uop::Swi(ib));
                    break;
                }
                IrOp::INT3 => {
                    self.uop_cache.push(Uop::Swi(3));
                    break;
                }
                IrOp::JCC_Jd(cc, target) => {
                    if let Some(addr_index) = self.address_map.get(&target).copied() {
                        self.uop_cache.push(Self::jcc_opt(cc, addr_index));
                    } else {
                        self.uop_cache.push(Uop::JccU(cc, target));
                    }
                    self.uop_cache.push(Uop::FetchNext(fetcher.current_eip()));
                    break;
                }
                IrOp::LOOP_Jd(addr32, target) => {
                    self.uop_cache
                        .push(Uop::Loop(LoopKind::Count, addr32, target));
                    self.uop_cache.push(Uop::FetchNext(fetcher.current_eip()));
                    break;
                }
                IrOp::LOOPZ_Jd(addr32, target) => {
                    self.uop_cache
                        .push(Uop::Loop(LoopKind::WhileZero, addr32, target));
                    self.uop_cache.push(Uop::FetchNext(fetcher.current_eip()));
                    break;
                }
                IrOp::LOOPNZ_Jd(addr32, target) => {
                    self.uop_cache
                        .push(Uop::Loop(LoopKind::WhileNotZero, addr32, target));
                    self.uop_cache.push(Uop::FetchNext(fetcher.current_eip()));
                    break;
                }
                IrOp::JCXZ_Jd(target) => {
                    self.uop_cache
                        .push(Uop::Loop(LoopKind::CountZero, false, target));
                    self.uop_cache.push(Uop::FetchNext(fetcher.current_eip()));
                    break;
                }
                IrOp::JECXZ_Jd(target) => {
                    self.uop_cache
                        .push(Uop::Loop(LoopKind::CountZero, true, target));
                    self.uop_cache.push(Uop::FetchNext(fetcher.current_eip()));
                    break;
                }
                IrOp::JMP_Jd(target) => {
                    if let Some(addr_index) = self.address_map.get(&target).copied() {
                        self.uop_cache.push(Uop::Jump(addr_index));
                        break;
                    } else {
                        fetcher.set_pos(target.0 as usize);
                    }
                }
                IrOp::JMP_Md(MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::JumpR(RtReg::MemData));
                    break;
                }
                IrOp::JMP_Rd(rd) => {
                    self.uop_cache.push(Uop::JumpR(rd.into()));
                    break;
                }
                IrOp::LAHF => self.uop_cache.push(Uop::Minor(UopMinor::Lahf)),
                IrOp::LEAVE => {
                    self.uop_cache.push(Uop::Minor(UopMinor::Leave));
                }
                IrOp::LEA_Rd_Md(rd, MemOpr::A32(ma)) => {
                    self.uop_cache.push(Self::generate_lea(rd.into(), ma));
                }
                IrOp::LEA_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    self.uop_cache.push(Self::generate_lea(RtReg::MemData, ma));
                    self.uop_cache
                        .push(Uop::Move16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::LODSB(opr) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::String(StringOp::new(
                            StringKind::Lods,
                            StringWidth::Byte,
                            StringRepeat::Once,
                            opr.segment,
                            opr.is_addr32,
                        ))));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::LODSD(opr) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::String(StringOp::new(
                            StringKind::Lods,
                            StringWidth::Dword,
                            StringRepeat::Once,
                            opr.segment,
                            opr.is_addr32,
                        ))));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::LODSW(opr) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::String(StringOp::new(
                            StringKind::Lods,
                            StringWidth::Word,
                            StringRepeat::Once,
                            opr.segment,
                            opr.is_addr32,
                        ))));
                    if !opr.is_addr32 {
                        break;
                    }
                }

                IrOp::MOVSB(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Movs,
                        StringWidth::Byte,
                        StringRepeat::Once,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::MOVSD(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Movs,
                        StringWidth::Dword,
                        StringRepeat::Once,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::MOVSW(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Movs,
                        StringWidth::Word,
                        StringRepeat::Once,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }

                IrOp::MOVSX_Rd_Mb(rd, MemOpr::A32(ma)) => {
                    if ma.segment != SrIndex::CS {
                        let base = Self::emit_lea_opt(&mut self.uop_cache, ma);
                        self.uop_cache
                            .push(Uop::LoadExtend8(LoadExtension::Sign, rd.into(), base));
                        continue;
                    }
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::MovSx8(rd.into(), RtReg8::MemData));
                }
                IrOp::MOVSX_Rd_Mw(rd, MemOpr::A32(ma)) => {
                    if ma.segment != SrIndex::CS {
                        let base = Self::emit_lea_opt(&mut self.uop_cache, ma);
                        self.uop_cache.push(Uop::LoadExtend16(
                            LoadExtension::Sign,
                            rd.into(),
                            base,
                        ));
                        continue;
                    }
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::MovSx16(rd.into(), RtReg::MemData));
                }
                IrOp::MOVSX_Rd_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::MovSx8(rd.into(), rs.into()));
                }
                IrOp::MOVSX_Rd_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::MovSx16(rd.into(), rs.upgrade().into()));
                }
                IrOp::MOVSX_Rw_Mb(rd, MemOpr::A32(ma)) | IrOp::MOVZX_Rw_Mb(rd, MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(match ir {
                        IrOp::MOVZX_Rw_Mb(..) => Uop::MovZx8(RtReg::MemData, RtReg8::MemData),
                        _ => Uop::MovSx8(RtReg::MemData, RtReg8::MemData),
                    });
                    self.uop_cache
                        .push(Uop::Move16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::MOVSX_Rw_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::MovSx8(RtReg::MemData, rs.into()));
                    self.uop_cache
                        .push(Uop::Move16(rd.upgrade().into(), RtReg::MemData));
                }

                IrOp::MOVZX_Rd_Mb(rd, MemOpr::A32(ma)) => {
                    if ma.segment != SrIndex::CS {
                        let base = Self::emit_lea_opt(&mut self.uop_cache, ma);
                        self.uop_cache
                            .push(Uop::LoadExtend8(LoadExtension::Zero, rd.into(), base));
                        continue;
                    }
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::MovZx8(rd.into(), RtReg8::MemData));
                }
                IrOp::MOVZX_Rd_Mw(rd, MemOpr::A32(ma)) => {
                    if ma.segment != SrIndex::CS {
                        let base = Self::emit_lea_opt(&mut self.uop_cache, ma);
                        self.uop_cache.push(Uop::LoadExtend16(
                            LoadExtension::Zero,
                            rd.into(),
                            base,
                        ));
                        continue;
                    }
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::MovZx16(rd.into(), RtReg::MemData));
                }
                IrOp::MOVZX_Rd_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::MovZx8(rd.into(), rs.into()));
                }
                IrOp::MOVZX_Rd_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::MovZx16(rd.into(), rs.upgrade().into()));
                }
                IrOp::MOVZX_Rw_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::MovZx8(RtReg::MemData, rs.into()));
                    self.uop_cache
                        .push(Uop::Move16(rd.upgrade().into(), RtReg::MemData));
                }

                IrOp::MOV_Mb_Ib(MemOpr::A32(ma), ib) => {
                    self.uop_cache.push(Uop::LoadConst8(RtReg8::MemData, ib));
                    let mem_addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    self.uop_cache.push(Uop::StoreR8(RtReg8::MemData, mem_addr));
                }
                IrOp::MOV_Mb_Rb(MemOpr::A32(ma), rb) => {
                    let mem_addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    self.uop_cache.push(Uop::StoreR8(rb.into(), mem_addr));
                }

                IrOp::MOV_Md_Id(MemOpr::A32(ma), id) => {
                    self.uop_cache.push(Uop::LoadConst(RtReg::MemData, id));
                    self.uop_cache
                        .push(Self::generate_store32(RtReg::MemData, ma));
                }
                IrOp::MOV_Md_Rd(MemOpr::A32(ma), rd) => {
                    self.uop_cache.push(Self::generate_store32(rd.into(), ma));
                }

                IrOp::MOV_Mw_Iw(MemOpr::A32(ma), iw) => {
                    self.uop_cache.push(Uop::LoadConst16(RtReg::MemData, iw));
                    let mem_addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    self.uop_cache.push(Uop::StoreR16(RtReg::MemData, mem_addr));
                }
                IrOp::MOV_Mw_Rw(MemOpr::A32(ma), rw) => {
                    let mem_addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    self.uop_cache
                        .push(Uop::StoreR16(rw.upgrade().into(), mem_addr));
                }
                IrOp::MOV_Rb_Ib(rd, ib) => {
                    self.uop_cache.push(Uop::LoadConst8(rd.into(), ib));
                }
                IrOp::MOV_Rb_Mb(rd, MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, rd.into(), ma);
                }
                IrOp::MOV_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::Move8(rd.into(), rs.into()));
                }
                IrOp::MOV_Rd_Id(rd, id) => {
                    self.uop_cache.push(Uop::LoadConst(rd.into(), id));
                }
                IrOp::MOV_Rd_Md(rd, MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, rd.into(), ma);
                }
                IrOp::MOV_Rd_Rd(rd, rs) => {
                    if rd != rs {
                        self.uop_cache.push(Uop::Move(rd.into(), rs.into()));
                    }
                }
                IrOp::MOV_Rw_Iw(rd, iw) => {
                    self.uop_cache
                        .push(Uop::LoadConst16(rd.upgrade().into(), iw));
                }
                IrOp::MOV_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, rd.upgrade().into(), ma);
                }
                IrOp::MOV_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::Move16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::MUL_Mb(MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::MulR8(RtReg8::MemData));
                }
                IrOp::MUL_Md(MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::MulR(RtReg::MemData));
                }
                IrOp::MUL_Mw(MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::MulR16(RtReg::MemData));
                }
                IrOp::MUL_Rb(rs) => {
                    self.uop_cache.push(Uop::MulR8(rs.into()));
                }
                IrOp::MUL_Rd(rs) => {
                    self.uop_cache.push(Uop::MulR(rs.into()));
                }
                IrOp::MUL_Rw(rs) => {
                    self.uop_cache.push(Uop::MulR16(rs.upgrade().into()));
                }
                IrOp::NEG_Mb(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::UnaryRMW(UnaryRmwOp::Neg8, addr)
                    });
                }
                IrOp::NEG_Md(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::UnaryRMW(UnaryRmwOp::Neg, addr)
                    });
                }
                IrOp::NEG_Mw(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::UnaryRMW(UnaryRmwOp::Neg16, addr)
                    });
                }
                IrOp::NEG_Rb(rd) => {
                    self.uop_cache.push(Uop::NegR8(rd.into()));
                }
                IrOp::NEG_Rd(rd) => {
                    self.uop_cache.push(Uop::NegR(rd.into()));
                }
                IrOp::NEG_Rw(rd) => {
                    self.uop_cache.push(Uop::NegR16(rd.upgrade().into()));
                }
                IrOp::NOP | IrOp::PAUSE => {
                    self.uop_cache.push(Uop::Nop);
                }
                IrOp::NOT_Mb(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::UnaryRMW(UnaryRmwOp::Not8, addr)
                    });
                }
                IrOp::NOT_Md(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::UnaryRMW(UnaryRmwOp::Not, addr)
                    });
                }
                IrOp::NOT_Mw(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::UnaryRMW(UnaryRmwOp::Not16, addr)
                    });
                }
                IrOp::NOT_Rb(rd) => {
                    self.uop_cache.push(Uop::Move8(RtReg8::MemData, rd.into()));
                    self.uop_cache.push(Uop::NotR(RtReg::MemData));
                    self.uop_cache.push(Uop::Move8(rd.into(), RtReg8::MemData));
                }
                IrOp::NOT_Rd(rd) => {
                    self.uop_cache.push(Uop::NotR(rd.into()));
                }
                IrOp::NOT_Rw(rd) => {
                    self.uop_cache
                        .push(Uop::Move16(RtReg::MemData, rd.upgrade().into()));
                    self.uop_cache.push(Uop::NotR(RtReg::MemData));
                    self.uop_cache
                        .push(Uop::Move16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::OR_Mb_Ib(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW8(BinaryRmwKind::Or, addr, imm)
                    });
                }
                IrOp::OR_Mb_Rb(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW8(BinaryRmwKind::Or, addr, rs.into())
                    });
                }
                IrOp::OR_Md_Id(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW(BinaryRmwKind::Or, addr, imm)
                    });
                }
                IrOp::OR_Md_Rd(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::OrRMW(addr, rs.into())
                    });
                }
                IrOp::OR_Mw_Iw(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW16(BinaryRmwKind::Or, addr, imm)
                    });
                }
                IrOp::OR_Mw_Rw(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW16(BinaryRmwKind::Or, addr, rs.upgrade().into())
                    });
                }
                IrOp::OR_Rb_Ib(rd, imm) => {
                    self.uop_cache.push(Uop::OrI8(rd.into(), imm));
                }
                IrOp::OR_Rb_Mb(rd, MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::OrR8(rd.into(), RtReg8::MemData));
                }
                IrOp::OR_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::OrR8(rd.into(), rs.into()));
                }
                IrOp::OR_Rd_Id(rd, imm) => {
                    self.uop_cache.push(Uop::OrI(rd.into(), imm));
                }
                IrOp::OR_Rd_Md(rd, MemOpr::A32(ma)) => {
                    let addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    self.uop_cache
                        .push(Uop::OrLoad(rd.into(), addr, ma.segment));
                }
                IrOp::OR_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::OrR(rd.into(), rs.into()));
                }
                IrOp::OR_Rw_Iw(rd, imm) => {
                    self.uop_cache.push(Uop::OrI16(rd.upgrade().into(), imm));
                }
                IrOp::OR_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::OrR16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::OR_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::OrR16(rd.upgrade().into(), rs.upgrade().into()));
                }

                IrOp::POPAD => {
                    self.uop_cache.push(Uop::Minor(UopMinor::PopAd));
                }
                IrOp::POPCNT_Rd_Md(rd, MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::Popcnt(rd.into(), RtReg::MemData));
                }
                IrOp::POPCNT_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::Popcnt(rd.into(), rs.into()))
                }
                IrOp::POPCNT_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::Popcnt16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::POPCNT_Rw_Rw(rd, rs) => self
                    .uop_cache
                    .push(Uop::Popcnt16(rd.upgrade().into(), rs.upgrade().into())),

                IrOp::POPFD => self.uop_cache.push(Uop::Minor(UopMinor::PopFd)),
                IrOp::POP_Md(MemOpr::A32(ma)) => {
                    let code = ma.segment == SrIndex::CS;
                    self.uop_cache.push(match ma.base_index {
                        BaseIndex32::DispOnly => Uop::PopMemR(RtReg::Zero, ma.disp, code),
                        BaseIndex32::Base(base) => Uop::PopMemR(base.into(), ma.disp, code),
                        BaseIndex32::Sib(sib) => Uop::PopMemSIB(sib, ma.disp, code),
                    });
                }

                IrOp::POP_Rd(rd) => {
                    self.uop_cache.push(Uop::PopR(rd.into()));
                }

                IrOp::PUSHAD => {
                    self.uop_cache.push(Uop::Minor(UopMinor::PushAd));
                }
                IrOp::PUSHFD => self.uop_cache.push(Uop::Minor(UopMinor::PushFd)),
                IrOp::PUSH_Id(id) => {
                    self.uop_cache.push(Uop::PushI(id));
                }
                IrOp::PUSH_Md(MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::PushR(RtReg::MemData));
                }
                IrOp::PUSH_Rd(rd) => {
                    self.uop_cache.push(Uop::PushR(rd.into()));
                }
                IrOp::RCL_Mb_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::Minor(UopMinor::RclRCl8(RtReg8::MemData)),
                        8,
                    );
                }
                IrOp::RCL_Mb_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::Minor(UopMinor::RclI8(RtReg8::MemData, count)),
                        8,
                    );
                }
                IrOp::RCL_Md_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::Minor(UopMinor::RclRCl(RtReg::MemData)),
                        32,
                    );
                }
                IrOp::RCL_Md_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::Minor(UopMinor::RclI(RtReg::MemData, count)),
                        32,
                    );
                }
                IrOp::RCL_Mw_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::Minor(UopMinor::RclRCl16(RtReg::MemData)),
                        16,
                    );
                }
                IrOp::RCL_Mw_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::Minor(UopMinor::RclI16(RtReg::MemData, count)),
                        16,
                    );
                }
                IrOp::RCL_Rb_Cl(rd) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::RclRCl8(rd.into())));
                }
                IrOp::RCL_Rb_Ib(rd, count) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::RclI8(rd.into(), count)));
                }
                IrOp::RCL_Rd_Cl(rd) => {
                    self.uop_cache.push(Uop::Minor(UopMinor::RclRCl(rd.into())));
                }
                IrOp::RCL_Rd_Ib(rd, count) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::RclI(rd.into(), count)));
                }
                IrOp::RCL_Rw_Cl(rd) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::RclRCl16(rd.upgrade().into())));
                }
                IrOp::RCL_Rw_Ib(rd, count) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::RclI16(rd.upgrade().into(), count)));
                }
                IrOp::RCR_Mb_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::Minor(UopMinor::RcrRCl8(RtReg8::MemData)),
                        8,
                    );
                }
                IrOp::RCR_Mb_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::Minor(UopMinor::RcrI8(RtReg8::MemData, count)),
                        8,
                    );
                }
                IrOp::RCR_Md_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::Minor(UopMinor::RcrRCl(RtReg::MemData)),
                        32,
                    );
                }
                IrOp::RCR_Md_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::Minor(UopMinor::RcrI(RtReg::MemData, count)),
                        32,
                    );
                }
                IrOp::RCR_Mw_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::Minor(UopMinor::RcrRCl16(RtReg::MemData)),
                        16,
                    );
                }
                IrOp::RCR_Mw_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::Minor(UopMinor::RcrI16(RtReg::MemData, count)),
                        16,
                    );
                }
                IrOp::RCR_Rb_Cl(rd) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::RcrRCl8(rd.into())));
                }
                IrOp::RCR_Rb_Ib(rd, count) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::RcrI8(rd.into(), count)));
                }
                IrOp::RCR_Rd_Cl(rd) => {
                    self.uop_cache.push(Uop::Minor(UopMinor::RcrRCl(rd.into())));
                }
                IrOp::RCR_Rd_Ib(rd, count) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::RcrI(rd.into(), count)));
                }
                IrOp::RCR_Rw_Cl(rd) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::RcrRCl16(rd.upgrade().into())));
                }
                IrOp::RCR_Rw_Ib(rd, count) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::RcrI16(rd.upgrade().into(), count)));
                }

                IrOp::RDTSC => {
                    self.uop_cache.push(Uop::Minor(UopMinor::RdTsc));
                }
                IrOp::REPNZ_CMPSB(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Cmps,
                        StringWidth::Byte,
                        StringRepeat::WhileNotZero,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REPNZ_CMPSD(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Cmps,
                        StringWidth::Dword,
                        StringRepeat::WhileNotZero,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REPNZ_CMPSW(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Cmps,
                        StringWidth::Word,
                        StringRepeat::WhileNotZero,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REPNZ_SCASB(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Scas,
                        StringWidth::Byte,
                        StringRepeat::WhileNotZero,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REPNZ_SCASD(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Scas,
                        StringWidth::Dword,
                        StringRepeat::WhileNotZero,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REPNZ_SCASW(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Scas,
                        StringWidth::Word,
                        StringRepeat::WhileNotZero,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REPZ_CMPSB(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Cmps,
                        StringWidth::Byte,
                        StringRepeat::WhileZero,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REPZ_CMPSD(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Cmps,
                        StringWidth::Dword,
                        StringRepeat::WhileZero,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REPZ_CMPSW(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Cmps,
                        StringWidth::Word,
                        StringRepeat::WhileZero,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REPZ_SCASB(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Scas,
                        StringWidth::Byte,
                        StringRepeat::WhileZero,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REPZ_SCASD(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Scas,
                        StringWidth::Dword,
                        StringRepeat::WhileZero,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REPZ_SCASW(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Scas,
                        StringWidth::Word,
                        StringRepeat::WhileZero,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REP_LODSB(opr) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::String(StringOp::new(
                            StringKind::Lods,
                            StringWidth::Byte,
                            StringRepeat::Count,
                            opr.segment,
                            opr.is_addr32,
                        ))));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REP_LODSD(opr) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::String(StringOp::new(
                            StringKind::Lods,
                            StringWidth::Dword,
                            StringRepeat::Count,
                            opr.segment,
                            opr.is_addr32,
                        ))));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REP_LODSW(opr) => {
                    self.uop_cache
                        .push(Uop::Minor(UopMinor::String(StringOp::new(
                            StringKind::Lods,
                            StringWidth::Word,
                            StringRepeat::Count,
                            opr.segment,
                            opr.is_addr32,
                        ))));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REP_MOVSB(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Movs,
                        StringWidth::Byte,
                        StringRepeat::Count,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REP_MOVSD(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Movs,
                        StringWidth::Dword,
                        StringRepeat::Count,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REP_MOVSW(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Movs,
                        StringWidth::Word,
                        StringRepeat::Count,
                        opr.segment,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REP_STOSB(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Stos,
                        StringWidth::Byte,
                        StringRepeat::Count,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REP_STOSD(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Stos,
                        StringWidth::Dword,
                        StringRepeat::Count,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::REP_STOSW(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Stos,
                        StringWidth::Word,
                        StringRepeat::Count,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }

                IrOp::RET_D32(iw) => {
                    self.uop_cache.push(Uop::Ret(iw));
                    break;
                }
                IrOp::ROL_Mb_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Rol8, addr)
                    });
                }
                IrOp::ROL_Mb_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftIRMW(ShiftRmwOp::Rol8, addr, count)
                    });
                }
                IrOp::ROL_Md_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Rol, addr)
                    });
                }
                IrOp::ROL_Md_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        if matches!(ma.base_index, BaseIndex32::DispOnly) {
                            Uop::ShiftIRMW(ShiftRmwOp::Rol, addr, count)
                        } else {
                            Uop::RolIRMW(addr, count)
                        }
                    });
                }
                IrOp::ROL_Mw_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Rol16, addr)
                    });
                }
                IrOp::ROL_Mw_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftIRMW(ShiftRmwOp::Rol16, addr, count)
                    });
                }
                IrOp::ROL_Rb_Cl(rd) => {
                    self.uop_cache.push(Uop::RolRCl8(rd.into()));
                }
                IrOp::ROL_Rb_Ib(rd, count) => {
                    self.uop_cache.push(Uop::RolI8(rd.into(), count));
                }
                IrOp::ROL_Rd_Cl(rd) => {
                    self.uop_cache.push(Uop::RolRCl(rd.into()));
                }
                IrOp::ROL_Rd_Ib(rd, count) => {
                    self.uop_cache.push(Uop::RolI(rd.into(), count));
                }
                IrOp::ROL_Rw_Cl(rd) => {
                    self.uop_cache.push(Uop::RolRCl16(rd.upgrade().into()));
                }
                IrOp::ROL_Rw_Ib(rd, count) => {
                    self.uop_cache.push(Uop::RolI16(rd.upgrade().into(), count));
                }
                IrOp::ROR_Mb_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Ror8, addr)
                    });
                }
                IrOp::ROR_Mb_Ib(MemOpr::A32(ma), count) => {
                    // The shared single-Uop kernel regresses on E-core Wasm here;
                    // dedicated variants also regress the common CPU dispatcher.
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::RorI8(RtReg8::MemData, count),
                        8,
                    );
                }
                IrOp::ROR_Md_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Ror, addr)
                    });
                }
                IrOp::ROR_Md_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftIRMW(ShiftRmwOp::Ror, addr, count)
                    });
                }
                IrOp::ROR_Mw_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Ror16, addr)
                    });
                }
                IrOp::ROR_Mw_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftIRMW(ShiftRmwOp::Ror16, addr, count)
                    });
                }
                IrOp::ROR_Rb_Cl(rd) => {
                    self.uop_cache.push(Uop::RorRCl8(rd.into()));
                }
                IrOp::ROR_Rb_Ib(rd, count) => {
                    self.uop_cache.push(Uop::RorI8(rd.into(), count));
                }
                IrOp::ROR_Rd_Cl(rd) => {
                    self.uop_cache.push(Uop::RorRCl(rd.into()));
                }
                IrOp::ROR_Rd_Ib(rd, count) => {
                    self.uop_cache.push(Uop::RorI(rd.into(), count));
                }
                IrOp::ROR_Rw_Cl(rd) => {
                    self.uop_cache.push(Uop::RorRCl16(rd.upgrade().into()));
                }
                IrOp::ROR_Rw_Ib(rd, count) => {
                    self.uop_cache.push(Uop::RorI16(rd.upgrade().into(), count));
                }
                IrOp::SAHF => self.uop_cache.push(Uop::Minor(UopMinor::Sahf)),

                IrOp::SAR_Mb_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Sar8, addr)
                    });
                }
                IrOp::SAR_Mb_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftIRMW(ShiftRmwOp::Sar8, addr, count)
                    });
                }
                IrOp::SAR_Md_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Sar, addr)
                    });
                }
                IrOp::SAR_Md_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftIRMW(ShiftRmwOp::Sar, addr, count)
                    });
                }
                IrOp::SAR_Mw_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Sar16, addr)
                    });
                }
                IrOp::SAR_Mw_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftIRMW(ShiftRmwOp::Sar16, addr, count)
                    });
                }
                IrOp::SAR_Rb_Cl(rd) => {
                    self.uop_cache.push(Uop::SarRCl8(rd.into()));
                }
                IrOp::SAR_Rb_Ib(rd, count) => {
                    self.uop_cache.push(Uop::SarI8(rd.into(), count));
                }
                IrOp::SAR_Rd_Cl(rd) => {
                    self.uop_cache.push(Uop::SarRCl(rd.into()));
                }
                IrOp::SAR_Rd_Ib(rd, count) => {
                    self.uop_cache.push(Uop::SarI(rd.into(), count));
                }
                IrOp::SAR_Rw_Cl(rd) => {
                    self.uop_cache.push(Uop::SarRCl16(rd.upgrade().into()));
                }
                IrOp::SAR_Rw_Ib(rd, count) => {
                    self.uop_cache.push(Uop::SarI16(rd.upgrade().into(), count));
                }
                IrOp::SBB_Mb_Ib(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW8(BinaryRmwKind::Sbb, addr, imm)
                    });
                }
                IrOp::SBB_Mb_Rb(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW8(BinaryRmwKind::Sbb, addr, rs.into())
                    });
                }
                IrOp::SBB_Md_Id(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW(BinaryRmwKind::Sbb, addr, imm)
                    });
                }
                IrOp::SBB_Md_Rd(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW(BinaryRmwKind::Sbb, addr, rs.into())
                    });
                }
                IrOp::SBB_Mw_Iw(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW16(BinaryRmwKind::Sbb, addr, imm)
                    });
                }
                IrOp::SBB_Mw_Rw(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW16(BinaryRmwKind::Sbb, addr, rs.upgrade().into())
                    });
                }
                IrOp::SBB_Rb_Ib(rd, imm) => {
                    self.uop_cache.push(Uop::SbbI8(rd.into(), imm));
                }
                IrOp::SBB_Rb_Mb(rd, MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::SbbR8(rd.into(), RtReg8::MemData));
                }
                IrOp::SBB_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::SbbR8(rd.into(), rs.into()));
                }
                IrOp::SBB_Rd_Id(rd, imm) => {
                    self.uop_cache.push(Uop::SbbI(rd.into(), imm));
                }
                IrOp::SBB_Rd_Md(rd, MemOpr::A32(ma)) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::SbbR(rd.into(), RtReg::MemData));
                }
                IrOp::SBB_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::SbbR(rd.into(), rs.into()));
                }
                IrOp::SBB_Rw_Iw(rd, imm) => {
                    self.uop_cache.push(Uop::SbbI16(rd.upgrade().into(), imm));
                }
                IrOp::SBB_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::SbbR16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::SBB_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::SbbR16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::SCASB(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Scas,
                        StringWidth::Byte,
                        StringRepeat::Once,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::SCASD(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Scas,
                        StringWidth::Dword,
                        StringRepeat::Once,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::SCASW(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Scas,
                        StringWidth::Word,
                        StringRepeat::Once,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::SETCC_Mb(cc, MemOpr::A32(ma)) => {
                    let addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    if ma.segment == SrIndex::CS {
                        self.uop_cache
                            .push(Uop::Minor(UopMinor::WriteCodeFault(addr)));
                    } else {
                        self.uop_cache.push(Uop::SetCC(cc, RtReg8::MemData));
                        self.uop_cache.push(Uop::StoreR8(RtReg8::MemData, addr));
                    }
                }

                IrOp::SETCC_Rb(cc, rd) => {
                    self.uop_cache.push(Uop::SetCC(cc, rd.into()));
                }
                IrOp::SHLD_Md_Rd_Cl(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::ShldRCl(RtReg::MemData, rs.into()),
                        32,
                    );
                }
                IrOp::SHLD_Md_Rd_Ib(MemOpr::A32(ma), rs, count) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::ShldI(RtReg::MemData, rs.into(), count),
                        32,
                    );
                }
                IrOp::SHLD_Mw_Rw_Cl(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::ShldRCl16(RtReg::MemData, rs.upgrade().into()),
                        16,
                    );
                }
                IrOp::SHLD_Mw_Rw_Ib(MemOpr::A32(ma), rs, count) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::ShldI16(RtReg::MemData, rs.upgrade().into(), count),
                        16,
                    );
                }
                IrOp::SHLD_Rd_Rd_Cl(rd, rs) => {
                    self.uop_cache.push(Uop::ShldRCl(rd.into(), rs.into()));
                }
                IrOp::SHLD_Rd_Rd_Ib(rd, rs, count) => {
                    self.uop_cache.push(Uop::ShldI(rd.into(), rs.into(), count));
                }
                IrOp::SHLD_Rw_Rw_Cl(rd, rs) => {
                    self.uop_cache
                        .push(Uop::ShldRCl16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::SHLD_Rw_Rw_Ib(rd, rs, count) => {
                    self.uop_cache.push(Uop::ShldI16(
                        rd.upgrade().into(),
                        rs.upgrade().into(),
                        count,
                    ));
                }
                IrOp::SHL_Mb_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Shl8, addr)
                    });
                }
                IrOp::SHL_Mb_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftIRMW(ShiftRmwOp::Shl8, addr, count)
                    });
                }
                IrOp::SHL_Md_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Shl, addr)
                    });
                }
                IrOp::SHL_Md_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftIRMW(ShiftRmwOp::Shl, addr, count)
                    });
                }
                IrOp::SHL_Mw_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Shl16, addr)
                    });
                }
                IrOp::SHL_Mw_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftIRMW(ShiftRmwOp::Shl16, addr, count)
                    });
                }
                IrOp::SHL_Rb_Cl(rd) => {
                    self.uop_cache.push(Uop::ShlRCl8(rd.into()));
                }
                IrOp::SHL_Rb_Ib(rd, count) => {
                    self.uop_cache.push(Uop::ShlI8(rd.into(), count));
                }
                IrOp::SHL_Rd_Cl(rd) => {
                    self.uop_cache.push(Uop::ShlRCl(rd.into()));
                }
                IrOp::SHL_Rd_Ib(rd, count) => {
                    self.uop_cache.push(Uop::ShlI(rd.into(), count));
                }
                IrOp::SHL_Rw_Cl(rd) => {
                    self.uop_cache.push(Uop::ShlRCl16(rd.upgrade().into()));
                }
                IrOp::SHL_Rw_Ib(rd, count) => {
                    self.uop_cache.push(Uop::ShlI16(rd.upgrade().into(), count));
                }
                IrOp::SHRD_Md_Rd_Cl(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::ShrdRCl(RtReg::MemData, rs.into()),
                        32,
                    );
                }
                IrOp::SHRD_Md_Rd_Ib(MemOpr::A32(ma), rs, count) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::ShrdI(RtReg::MemData, rs.into(), count),
                        32,
                    );
                }
                IrOp::SHRD_Mw_Rw_Cl(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::ShrdRCl16(RtReg::MemData, rs.upgrade().into()),
                        16,
                    );
                }
                IrOp::SHRD_Mw_Rw_Ib(MemOpr::A32(ma), rs, count) => {
                    Self::emit_memory_fallback(
                        &mut self.uop_cache,
                        ma,
                        Uop::ShrdI16(RtReg::MemData, rs.upgrade().into(), count),
                        16,
                    );
                }
                IrOp::SHRD_Rd_Rd_Cl(rd, rs) => {
                    self.uop_cache.push(Uop::ShrdRCl(rd.into(), rs.into()));
                }
                IrOp::SHRD_Rd_Rd_Ib(rd, rs, count) => {
                    self.uop_cache.push(Uop::ShrdI(rd.into(), rs.into(), count));
                }
                IrOp::SHRD_Rw_Rw_Cl(rd, rs) => {
                    self.uop_cache
                        .push(Uop::ShrdRCl16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::SHRD_Rw_Rw_Ib(rd, rs, count) => {
                    self.uop_cache.push(Uop::ShrdI16(
                        rd.upgrade().into(),
                        rs.upgrade().into(),
                        count,
                    ));
                }
                IrOp::SHR_Mb_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Shr8, addr)
                    });
                }
                IrOp::SHR_Mb_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftIRMW(ShiftRmwOp::Shr8, addr, count)
                    });
                }
                IrOp::SHR_Md_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Shr, addr)
                    });
                }
                IrOp::SHR_Md_Ib(MemOpr::A32(ma), count) => {
                    if matches!(ma.base_index, BaseIndex32::Sib(_)) {
                        Self::emit_memory_fallback(
                            &mut self.uop_cache,
                            ma,
                            Uop::ShrI(RtReg::MemData, count),
                            32,
                        );
                    } else {
                        Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                            Uop::ShiftIRMW(ShiftRmwOp::Shr, addr, count)
                        });
                    }
                }
                IrOp::SHR_Mw_Cl(MemOpr::A32(ma)) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftRMWCl(ShiftRmwOp::Shr16, addr)
                    });
                }
                IrOp::SHR_Mw_Ib(MemOpr::A32(ma), count) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::ShiftIRMW(ShiftRmwOp::Shr16, addr, count)
                    });
                }
                IrOp::SHR_Rb_Cl(rd) => {
                    self.uop_cache.push(Uop::ShrRCl8(rd.into()));
                }
                IrOp::SHR_Rb_Ib(rd, count) => {
                    self.uop_cache.push(Uop::ShrI8(rd.into(), count));
                }
                IrOp::SHR_Rd_Cl(rd) => {
                    self.uop_cache.push(Uop::ShrRCl(rd.into()));
                }
                IrOp::SHR_Rd_Ib(rd, count) => {
                    self.uop_cache.push(Uop::ShrI(rd.into(), count));
                }
                IrOp::SHR_Rw_Cl(rd) => {
                    self.uop_cache.push(Uop::ShrRCl16(rd.upgrade().into()));
                }
                IrOp::SHR_Rw_Ib(rd, count) => {
                    self.uop_cache.push(Uop::ShrI16(rd.upgrade().into(), count));
                }
                IrOp::STC => self.uop_cache.push(Uop::Minor(UopMinor::Stc)),
                IrOp::STD => {
                    self.uop_cache.push(Uop::Minor(UopMinor::Std));
                }
                IrOp::STOSB(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Stos,
                        StringWidth::Byte,
                        StringRepeat::Once,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::STOSD(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Stos,
                        StringWidth::Dword,
                        StringRepeat::Once,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::STOSW(opr) => {
                    self.uop_cache.push(Uop::String(StringOp::new(
                        StringKind::Stos,
                        StringWidth::Word,
                        StringRepeat::Once,
                        SrIndex::ES,
                        opr.is_addr32,
                    )));
                    if !opr.is_addr32 {
                        break;
                    }
                }
                IrOp::SUB_Mb_Ib(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW8(BinaryRmwKind::Sub, addr, imm)
                    });
                }
                IrOp::SUB_Mb_Rb(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::SubRMW8(addr, rs.into())
                    });
                }
                IrOp::SUB_Md_Id(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW(BinaryRmwKind::Sub, addr, imm)
                    });
                }
                IrOp::SUB_Md_Rd(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::SubRMW(addr, rs.into())
                    });
                }
                IrOp::SUB_Mw_Iw(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW16(BinaryRmwKind::Sub, addr, imm)
                    });
                }
                IrOp::SUB_Mw_Rw(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW16(BinaryRmwKind::Sub, addr, rs.upgrade().into())
                    });
                }
                IrOp::SUB_Rb_Ib(rd, imm) => {
                    self.uop_cache.push(Uop::SubI8(rd.into(), imm));
                }
                IrOp::SUB_Rb_Mb(rd, MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::SubR8(rd.into(), RtReg8::MemData));
                }
                IrOp::SUB_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::SubR8(rd.into(), rs.into()));
                }
                IrOp::SUB_Rd_Id(rd, imm) => {
                    self.uop_cache.push(Uop::SubI(rd.into(), imm));
                }
                IrOp::SUB_Rd_Md(rd, MemOpr::A32(ma)) => {
                    let addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    self.uop_cache
                        .push(Uop::SubLoad(rd.into(), addr, ma.segment));
                }
                IrOp::SUB_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::SubR(rd.into(), rs.into()));
                }
                IrOp::SUB_Rw_Iw(rd, imm) => {
                    self.uop_cache.push(Uop::SubI16(rd.upgrade().into(), imm));
                }
                IrOp::SUB_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::SubR16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::SUB_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::SubR16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::TEST_Mb_Ib(MemOpr::A32(ma), imm) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::TestI8(RtReg8::MemData, imm));
                }
                IrOp::TEST_Mb_Rb(MemOpr::A32(ma), rs) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::TestR8(RtReg8::MemData, rs.into()));
                }
                IrOp::TEST_Md_Id(MemOpr::A32(ma), imm) => {
                    Self::emit_load32(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::TestI(RtReg::MemData, imm));
                }
                IrOp::TEST_Md_Rd(MemOpr::A32(ma), rs) => {
                    let addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    self.uop_cache
                        .push(Uop::TestMemReg(addr, rs.into(), ma.segment));
                }
                IrOp::TEST_Mw_Iw(MemOpr::A32(ma), imm) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache.push(Uop::TestI16(RtReg::MemData, imm));
                }
                IrOp::TEST_Mw_Rw(MemOpr::A32(ma), rs) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::TestR16(RtReg::MemData, rs.upgrade().into()));
                }
                IrOp::TEST_Rb_Ib(rd, imm) => {
                    self.uop_cache.push(Uop::TestI8(rd.into(), imm));
                }
                IrOp::TEST_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::TestR8(rd.into(), rs.into()));
                }
                IrOp::TEST_Rd_Id(rd, imm) => {
                    self.uop_cache.push(Uop::TestI(rd.into(), imm));
                }
                IrOp::TEST_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::TestR(rd.into(), rs.into()));
                }
                IrOp::TEST_Rw_Iw(rd, imm) => {
                    self.uop_cache.push(Uop::TestI16(rd.upgrade().into(), imm));
                }
                IrOp::TEST_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::TestR16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::UD => {
                    self.uop_cache.push(Uop::Minor(UopMinor::InvalidOpcode));
                    break;
                }
                IrOp::XADD_Mb_Rb(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::XaddRMW8(addr, rs.into())
                    });
                }
                IrOp::XADD_Md_Rd(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::XaddRMW(addr, rs.into())
                    });
                }
                IrOp::XADD_Mw_Rw(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::XaddRMW16(addr, rs.upgrade().into())
                    });
                }
                IrOp::XADD_Rb_Rb(rd, rs) => self.uop_cache.push(Uop::XaddR8(rd.into(), rs.into())),
                IrOp::XADD_Rd_Rd(rd, rs) => self.uop_cache.push(Uop::XaddR(rd.into(), rs.into())),
                IrOp::XADD_Rw_Rw(rd, rs) => self
                    .uop_cache
                    .push(Uop::XaddR16(rd.upgrade().into(), rs.upgrade().into())),
                IrOp::XCHG_Mb_Rb(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::XchgRMW8(addr, rs.into())
                    });
                }
                IrOp::XCHG_Md_Rd(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::XchgRMW(addr, rs.into())
                    });
                }
                IrOp::XCHG_Mw_Rw(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::XchgRMW16(addr, rs.upgrade().into())
                    });
                }
                IrOp::XCHG_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::XchgR8(rd.into(), rs.into()));
                }

                IrOp::XCHG_Rd_Rd(rd, rs) => {
                    if rd != rs {
                        self.uop_cache.push(Uop::XchgR(rd.into(), rs.into()));
                    }
                }
                IrOp::XCHG_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::XchgR16(rd.upgrade().into(), rs.upgrade().into()));
                }
                IrOp::XLAT_Mb(MemOpr::A32(ma)) => {
                    self.uop_cache.push(Uop::Minor(UopMinor::Xlat(ma.segment)))
                }
                IrOp::XOR_Mb_Ib(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW8(BinaryRmwKind::Xor, addr, imm)
                    });
                }
                IrOp::XOR_Mb_Rb(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW8(BinaryRmwKind::Xor, addr, rs.into())
                    });
                }
                IrOp::XOR_Md_Id(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW(BinaryRmwKind::Xor, addr, imm)
                    });
                }
                IrOp::XOR_Md_Rd(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::XorRMW(addr, rs.into())
                    });
                }
                IrOp::XOR_Mw_Iw(MemOpr::A32(ma), imm) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryIRMW16(BinaryRmwKind::Xor, addr, imm)
                    });
                }
                IrOp::XOR_Mw_Rw(MemOpr::A32(ma), rs) => {
                    Self::emit_memory_update(&mut self.uop_cache, ma, |addr| {
                        Uop::BinaryRMW16(BinaryRmwKind::Xor, addr, rs.upgrade().into())
                    });
                }
                IrOp::XOR_Rb_Ib(rd, imm) => {
                    self.uop_cache.push(Uop::XorI8(rd.into(), imm));
                }
                IrOp::XOR_Rb_Mb(rd, MemOpr::A32(ma)) => {
                    Self::emit_load8(&mut self.uop_cache, RtReg8::MemData, ma);
                    self.uop_cache.push(Uop::XorR8(rd.into(), RtReg8::MemData));
                }
                IrOp::XOR_Rb_Rb(rd, rs) => {
                    self.uop_cache.push(Uop::XorR8(rd.into(), rs.into()));
                }
                IrOp::XOR_Rd_Id(rd, imm) => {
                    self.uop_cache.push(Uop::XorI(rd.into(), imm));
                }
                IrOp::XOR_Rd_Md(rd, MemOpr::A32(ma)) => {
                    let addr = Self::emit_lea_opt(&mut self.uop_cache, ma);
                    self.uop_cache
                        .push(Uop::XorLoad(rd.into(), addr, ma.segment));
                }
                IrOp::XOR_Rd_Rd(rd, rs) => {
                    self.uop_cache.push(Uop::XorR(rd.into(), rs.into()));
                }
                IrOp::XOR_Rw_Iw(rd, imm) => {
                    self.uop_cache.push(Uop::XorI16(rd.upgrade().into(), imm));
                }
                IrOp::XOR_Rw_Mw(rd, MemOpr::A32(ma)) => {
                    Self::emit_load16(&mut self.uop_cache, RtReg::MemData, ma);
                    self.uop_cache
                        .push(Uop::XorR16(rd.upgrade().into(), RtReg::MemData));
                }
                IrOp::XOR_Rw_Rw(rd, rs) => {
                    self.uop_cache
                        .push(Uop::XorR16(rd.upgrade().into(), rs.upgrade().into()));
                }

                _ => {
                    self.uop_cache.push(Uop::Minor(UopMinor::Unsupported(
                        UnsupportedKind::NoLowering,
                    )));
                }
            }
        }
        self.eip_to_fetch = fetcher.current_eip();
        #[cfg(feature = "flag-elision-analysis")]
        {
            crate::elision::analyze(&effects, |index| {
                #[cfg(feature = "uop-profile")]
                if let Some((_, ir)) = self.profile_starts.get_mut(&index) {
                    ir.push_str(" [candidate]");
                }
                #[cfg(feature = "flag-elision")]
                super::elide_uop::rewrite(&mut self.uop_cache[index as usize]);
                #[cfg(not(any(feature = "flag-elision", feature = "uop-profile")))]
                core::hint::black_box(index);
            });
            effects.clear();
            self.effects = effects;
        }
    }
}
