//! CPU definitions and processor state

use core::mem::MaybeUninit;

use ir86::prelude::registers::*;

use crate::flags::{FlagsRegister, LazyOp};
use crate::gpr::*;
use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Generation {
    // I8086 = 0,
    // I186 = 1,
    // I286 = 2,
    I386 = 3,
    I486 = 4,
    Pentium = 5,
    P6 = 6,
    P7 = 7,
    NetBurst = 15,
}

impl Generation {
    /// The latest CPU generation supported by this emulator.
    pub const LATEST: Generation = Generation::P6;
}

/// Processor State
///
/// `repr(C)` in every build: `gpr` must stay at offset 0.
#[derive(Clone)]
#[repr(C)]
pub struct ProcessorState {
    gpr: [GeneralPurposeRegister; 16],
    sr: [SegmentRegister; 8],

    #[cfg(feature = "x87")]
    pub(crate) fpu: crate::fpu::FpuState,

    flags: FlagsRegister,

    sys: SystemRegisters,
    cpuid: Cpuid,

    #[cfg(feature = "sse")]
    pub(crate) simd: crate::simd::SimdState,
}

const _: () = assert!(core::mem::offset_of!(ProcessorState, gpr) == 0);

#[cfg(feature = "sse")]
const _: () = {
    assert!(core::mem::align_of::<ProcessorState>() >= 16);
    assert!(core::mem::offset_of!(ProcessorState, simd) % 16 == 0);
};

/// GDTR/IDTR image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableRegister {
    pub base: Linear32,
    pub limit: Limit16,
}

impl TableRegister {
    /// Creates a new table register with the specified base and limit.
    #[inline]
    pub fn new(base: Linear32, limit: Limit16) -> Self {
        Self { base, limit }
    }
}

/// System registers used by the system-mode engine. User-mode engines leave
/// them at their zero value. LDTR/TR keep their selector and hidden descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemRegisters {
    pub cr0: u32,
    /// Linear address of the last page fault.
    pub cr2: Linear32,
    /// Raw CR3 image: physical page-directory base and PWT/PCD flags.
    pub cr3: u32,
    pub cr4: u32,
    pub gdtr: TableRegister,
    pub idtr: TableRegister,
    pub ldtr: SegmentRegister,
    pub tr: SegmentRegister,
    /// Current privilege level. Real mode keeps 0; CS loads update it.
    pub cpl: u8,
    /// DR0-DR3 as written. No breakpoint is ever matched.
    pub dr: [Linear32; 4],
    /// DR6 and DR7 without their fixed bits (DR6 reads with bits 4-11 and
    /// 16-31 set, DR7 with bit 10 set), so the zero state is the reset value.
    pub dr6: u32,
    pub dr7: u32,
    /// IA32_SYSENTER_CS/ESP/EIP (MSRs 174h-176h).
    pub sysenter_cs: SegmentSelector,
    pub sysenter_esp: Offset32,
    pub sysenter_eip: Offset32,
}

/// Runtime Register Index
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RtReg {
    // General-purpose registers
    EAX = 0,
    ECX = 1,
    EDX = 2,
    EBX = 3,
    ESP = 4,
    EBP = 5,
    ESI = 6,
    EDI = 7,

    /// EIP
    EIP = 8,
    /// Memory Effective Address
    MemAddr,
    /// Memory Temp Data
    MemData,
    /// GS Segment Base (mirrors `sr(GS).base` while executing)
    GsBase,
    /// Zero Register. Some instructions may use this as a source operand that always reads as zero.
    Zero = 15,
}

/// Runtime Register Index for 8-bit registers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RtReg8 {
    // General-purpose 8-bit registers
    AL = 0,
    CL = 1,
    DL = 2,
    BL = 3,
    AH = 4,
    CH = 5,
    DH = 6,
    BH = 7,

    /// Memory Temp Data
    MemData,
}

impl From<GprIndex32> for RtReg {
    fn from(index: GprIndex32) -> Self {
        match index {
            EAX => RtReg::EAX,
            ECX => RtReg::ECX,
            EDX => RtReg::EDX,
            EBX => RtReg::EBX,
            ESP => RtReg::ESP,
            EBP => RtReg::EBP,
            ESI => RtReg::ESI,
            EDI => RtReg::EDI,
        }
    }
}

impl From<GprIndex8> for RtReg8 {
    fn from(index: GprIndex8) -> Self {
        match index {
            AL => RtReg8::AL,
            CL => RtReg8::CL,
            DL => RtReg8::DL,
            BL => RtReg8::BL,
            AH => RtReg8::AH,
            CH => RtReg8::CH,
            DH => RtReg8::DH,
            BH => RtReg8::BH,
        }
    }
}

impl ProcessorState {
    /// Creates a new processor state with the specified CPU generation.
    #[inline]
    pub fn new(
        generation: Generation,
        manufacturer_string: &[u8; 12],
        brand_string: &[u8; 48],
    ) -> Self {
        let mut state = unsafe { MaybeUninit::<Self>::zeroed().assume_init() };
        state.init_with_generation(generation);
        state
            .cpuid
            .init(generation, manufacturer_string, brand_string);

        state
    }

    #[inline]
    pub fn clone_from(&mut self, other: &ProcessorState) {
        *self = other.clone();
    }

    #[inline]
    pub fn with_generation(generation: Generation) -> Self {
        Self::new(generation, &[0; 12], &[0; 48])
    }

    #[inline]
    pub fn latest() -> Self {
        Self::with_generation(Generation::LATEST)
    }

    /// Initializes the processor state based on the CPU generation.
    pub fn init_with_generation(&mut self, generation: Generation) {
        self.cpuid.set_generation(generation);
        self.init();
    }

    /// Initializes the processor state.
    pub fn init(&mut self) {
        self.flags = FlagsRegister::new(self.cpuid.generation());

        self.sr_mut(CS).init_code_rm();
        self.sr_mut(DS).init_data_rm();
        self.sr_mut(ES).init_data_rm();
        self.sr_mut(FS).init_data_rm();
        self.sr_mut(GS).init_data_rm();
        self.sr_mut(SS).init_data_rm();

        for gpr in &self.gpr {
            gpr.e().write(0);
        }
        self.eip().write(0x0000_fff0);

        let sys = self.sys_mut();
        sys.cpl = 0;
        sys.cr0 = 0x6000_0010;
        sys.cr2 = Linear32(0);
        sys.cr4 = 0;
        sys.gdtr = TableRegister::new(Linear32(0), Limit16::MAX);
        sys.idtr = TableRegister::new(Linear32(0), Limit16::MAX);
        sys.ldtr = SegmentRegister::new(SegmentSelector(0), Linear32(0), Limit32(0xffff), 0x82);
        sys.tr = SegmentRegister::new(SegmentSelector(0), Linear32(0), Limit32(0xffff), 0x8b);

        #[cfg(feature = "x87")]
        {
            self.fpu = crate::fpu::FpuState::default();
        }
        #[cfg(feature = "sse")]
        {
            self.simd = crate::simd::SimdState::default();
        }
    }

    /// Returns the CPU generation.
    #[inline]
    pub fn generation(&self) -> Generation {
        self.cpuid.generation()
    }

    /// Returns the runtime register at the specified index.
    #[inline(always)]
    pub fn reg(&self, index: RtReg) -> &GeneralPurposeRegister {
        &self.gpr[index as usize & 15]
    }

    /// Returns a partial register (8-bit) at the specified index.
    #[inline]
    pub fn reg8<'a>(&'a self, index: RtReg8) -> Gpr8<'a> {
        match index {
            RtReg8::AL => self.al(),
            RtReg8::CL => self.cl(),
            RtReg8::DL => self.dl(),
            RtReg8::BL => self.bl(),
            RtReg8::AH => self.ah(),
            RtReg8::CH => self.ch(),
            RtReg8::DH => self.dh(),
            RtReg8::BH => self.bh(),
            RtReg8::MemData => self.reg(RtReg::MemData).l(),
        }
    }

    /// Returns the FPU state.
    #[cfg(feature = "x87")]
    #[inline(always)]
    pub fn fpu(&self) -> &crate::fpu::FpuState {
        &self.fpu
    }

    /// Returns the XMM registers and MXCSR.
    #[cfg(feature = "sse")]
    #[inline(always)]
    pub fn simd(&self) -> &crate::simd::SimdState {
        &self.simd
    }

    /// Returns the XMM registers and MXCSR for modification.
    #[cfg(feature = "sse")]
    #[inline(always)]
    pub fn simd_mut(&mut self) -> &mut crate::simd::SimdState {
        &mut self.simd
    }

    /// Returns the system registers.
    #[inline(always)]
    pub fn sys(&self) -> &SystemRegisters {
        &self.sys
    }

    /// Returns the system registers for modification.
    #[inline(always)]
    pub fn sys_mut(&mut self) -> &mut SystemRegisters {
        &mut self.sys
    }

    /// Returns the segment register at the specified index.
    #[inline(always)]
    pub fn sr(&self, index: SrIndex) -> &SegmentRegister {
        &self.sr[index as usize & 7]
    }

    /// Returns a mutable reference to the segment register at the specified index.
    #[inline(always)]
    pub fn sr_mut(&mut self, index: SrIndex) -> &mut SegmentRegister {
        &mut self.sr[index as usize & 7]
    }

    #[inline]
    pub fn al<'a>(&'a self) -> Gpr8<'a> {
        self.reg(RtReg::EAX).l()
    }

    #[inline]
    pub fn ah<'a>(&'a self) -> Gpr8<'a> {
        self.reg(RtReg::EAX).h()
    }

    #[inline]
    pub fn ax<'a>(&'a self) -> Gpr16<'a> {
        self.reg(RtReg::EAX).x()
    }

    #[inline]
    pub fn eax<'a>(&'a self) -> Gpr32<'a> {
        self.reg(RtReg::EAX).e()
    }

    #[inline]
    pub fn bl<'a>(&'a self) -> Gpr8<'a> {
        self.reg(RtReg::EBX).l()
    }

    #[inline]
    pub fn bh<'a>(&'a self) -> Gpr8<'a> {
        self.reg(RtReg::EBX).h()
    }

    #[inline]
    pub fn bx<'a>(&'a self) -> Gpr16<'a> {
        self.reg(RtReg::EBX).x()
    }

    #[inline]
    pub fn ebx<'a>(&'a self) -> Gpr32<'a> {
        self.reg(RtReg::EBX).e()
    }

    #[inline]
    pub fn cl<'a>(&'a self) -> Gpr8<'a> {
        self.reg(RtReg::ECX).l()
    }

    #[inline]
    pub fn ch<'a>(&'a self) -> Gpr8<'a> {
        self.reg(RtReg::ECX).h()
    }

    #[inline]
    pub fn cx<'a>(&'a self) -> Gpr16<'a> {
        self.reg(RtReg::ECX).x()
    }

    #[inline]
    pub fn ecx<'a>(&'a self) -> Gpr32<'a> {
        self.reg(RtReg::ECX).e()
    }

    #[inline]
    pub fn dl<'a>(&'a self) -> Gpr8<'a> {
        self.reg(RtReg::EDX).l()
    }

    #[inline]
    pub fn dh<'a>(&'a self) -> Gpr8<'a> {
        self.reg(RtReg::EDX).h()
    }

    #[inline]
    pub fn dx<'a>(&'a self) -> Gpr16<'a> {
        self.reg(RtReg::EDX).x()
    }

    #[inline]
    pub fn edx<'a>(&'a self) -> Gpr32<'a> {
        self.reg(RtReg::EDX).e()
    }

    #[inline]
    pub fn sp<'a>(&'a self) -> Gpr16<'a> {
        self.reg(RtReg::ESP).x()
    }

    #[inline]
    pub fn esp<'a>(&'a self) -> Gpr32<'a> {
        self.reg(RtReg::ESP).e()
    }

    #[inline]
    pub fn bp<'a>(&'a self) -> Gpr16<'a> {
        self.reg(RtReg::EBP).x()
    }

    #[inline]
    pub fn ebp<'a>(&'a self) -> Gpr32<'a> {
        self.reg(RtReg::EBP).e()
    }

    #[inline]
    pub fn si<'a>(&'a self) -> Gpr16<'a> {
        self.reg(RtReg::ESI).x()
    }

    #[inline]
    pub fn esi<'a>(&'a self) -> Gpr32<'a> {
        self.reg(RtReg::ESI).e()
    }

    #[inline]
    pub fn di<'a>(&'a self) -> Gpr16<'a> {
        self.reg(RtReg::EDI).x()
    }

    #[inline]
    pub fn edi<'a>(&'a self) -> Gpr32<'a> {
        self.reg(RtReg::EDI).e()
    }

    #[inline]
    pub fn eip<'a>(&'a self) -> Gpr32<'a> {
        self.reg(RtReg::EIP).e()
    }

    #[inline]
    pub fn cs(&self) -> &SegmentRegister {
        self.sr(CS)
    }

    #[inline]
    pub fn ds(&self) -> &SegmentRegister {
        self.sr(DS)
    }

    #[inline]
    pub fn es(&self) -> &SegmentRegister {
        self.sr(ES)
    }

    #[inline]
    pub fn fs(&self) -> &SegmentRegister {
        self.sr(FS)
    }

    #[inline]
    pub fn gs(&self) -> &SegmentRegister {
        self.sr(GS)
    }

    #[inline]
    pub fn ss(&self) -> &SegmentRegister {
        self.sr(SS)
    }

    #[inline(always)]
    pub fn flags(&self) -> &FlagsRegister {
        &self.flags
    }

    #[inline(always)]
    pub fn flags_mut(&mut self) -> &mut FlagsRegister {
        &mut self.flags
    }

    /// Evaluates the condition code based on the current flags.
    #[inline(always)]
    pub fn eval_cc(&mut self, cc: CC) -> bool {
        self.flags.eval_cc(cc)
    }

    #[inline(always)]
    pub fn set_lazy_op(&mut self, lazy_op: LazyOp) {
        self.flags.set_lazy_op(lazy_op);
    }

    /// Adjusts the flags after generic operations (ADD, SUB, etc.).
    #[inline(always)]
    pub(crate) fn reset_flags_with_zf(&mut self, is_zero: bool) {
        self.flags.reset_flags_with_zf(is_zero);
    }

    /// Adjusts the flags after logic operations (AND, OR, XOR, etc.).
    #[inline(always)]
    pub(crate) fn adjust_after_logic_op(&mut self, is_zero: bool) {
        self.flags.adjust_after_logic_op(is_zero);
    }

    /// Adjusts the flags after INC and DEC operations; `cf` is the carry before the operation.
    #[inline(always)]
    pub(crate) fn adjust_after_inc_dec(&mut self, cf: bool, is_zero: bool) {
        self.flags.adjust_after_inc_dec(cf, is_zero);
    }

    /// Invokes the CPUID instruction
    #[inline]
    pub fn invoke_cpuid(&mut self) {
        let eax = self.eax().read();
        let ecx = self.ecx().read();
        let result = self.cpuid.invoke(eax, ecx);
        self.eax().write(result.eax);
        self.ebx().write(result.ebx);
        self.ecx().write(result.ecx);
        self.edx().write(result.edx);
    }

    #[inline(always)]
    pub fn cpuid(&self) -> &Cpuid {
        &self.cpuid
    }

    #[inline(always)]
    pub fn cpuid_mut(&mut self) -> &mut Cpuid {
        &mut self.cpuid
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Cpuid {
    generation: Generation,
    processor_version: u32,
    manufacturer_string: [u8; 12],
    brand_string: [u8; 48],

    processor_feature_01d: u32,
    processor_feature_01c: u32,
    // processor_feature_070b: u32,
    // processor_feature_070c: u32,
    // processor_feature_070d: u32,
    processor_feature_81d: u32,
    processor_feature_81c: u32,
}

/// Leaf 1 EDX bits the `sse` feature reports: CLFSH (19),
/// MMX (23), FXSR (24), SSE (25) and SSE2 (26).
#[cfg(feature = "sse")]
pub const CPUID_SIMD_01D: u32 = (1 << 19) | (1 << 23) | (1 << 24) | (1 << 25) | (1 << 26);
/// CLFLUSH line size in 8-byte units: 64 bytes.
#[cfg(feature = "sse")]
pub const CPUID_CLFLUSH_LINE_SIZE: u8 = 8;

#[derive(Debug, Clone, Copy)]
pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

impl Cpuid {
    #[inline]
    pub fn init(
        &mut self,
        generation: Generation,
        manufacturer_string: &[u8; 12],
        brand_string: &[u8; 48],
    ) {
        self.manufacturer_string
            .copy_from_slice(manufacturer_string);
        self.brand_string.copy_from_slice(brand_string);
        self.set_feature_01(0, 0);
        self.set_feature_81(0, 0);
        self.set_generation(generation);
    }

    pub fn set_generation(&mut self, generation: Generation) {
        self.generation = generation;

        let generation = self.generation() as u32;
        let family = if generation < 15 { generation } else { 15 };
        let model = 0;
        let stepping = 0;
        let efamily = if generation >= 15 { generation - 15 } else { 0 };
        let emodel = 0;
        self.processor_version =
            (efamily << 20) | (emodel << 16) | (family << 8) | (model << 4) | stepping;
    }

    /// Set processor feature flags for CPUID leaf 1.
    /// `feature_01d` corresponds to EDX and `feature_01c` corresponds to ECX.
    ///
    /// Notes:
    /// - The FPU flag (EDX bit 0) is set according to the `x87` feature.
    /// - Some bits are always set.
    #[inline]
    pub fn set_feature_01(&mut self, feature_01d: u32, feature_01c: u32) {
        // With `sse`, `invoke` always adds the whole SSE family.
        #[cfg(feature = "sse")]
        let feature_01d = feature_01d & !CPUID_SIMD_01D;
        let fpu_flag = if cfg!(feature = "x87") { 1 } else { 0 };
        self.processor_feature_01d |= feature_01d | 0x0000_8010 | fpu_flag;
        self.processor_feature_01c |= feature_01c | 0x0080_0000;
    }

    /// Clear processor feature flags for CPUID leaf 1.
    /// `feature_01d` corresponds to EDX and `feature_01c` corresponds to ECX.
    #[inline]
    pub fn clear_feature_01(&mut self, feature_01d: u32, feature_01c: u32) {
        self.processor_feature_01d &= !feature_01d;
        self.processor_feature_01c &= !feature_01c;
    }

    /// Set processor feature flags for CPUID leaf 0x8000_0001.
    /// `feature_81d` corresponds to EDX and `feature_81c` corresponds to ECX.
    #[inline]
    pub fn set_feature_81(&mut self, feature_81d: u32, feature_81c: u32) {
        self.processor_feature_81d = feature_81d;
        self.processor_feature_81c = feature_81c;
    }

    #[inline]
    pub fn generation(&self) -> Generation {
        self.generation
    }

    pub fn invoke(&self, eax: u32, _ecx: u32) -> CpuidResult {
        let mut result = CpuidResult {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
        };

        match eax {
            0x0000_0000 => {
                result.eax = 1;
                result.ebx = u32::from_le_bytes(self.manufacturer_string[0..4].try_into().unwrap());
                result.edx = u32::from_le_bytes(self.manufacturer_string[4..8].try_into().unwrap());
                result.ecx =
                    u32::from_le_bytes(self.manufacturer_string[8..12].try_into().unwrap());
            }
            0x0000_0001 => {
                result.eax = self.processor_version;
                result.ebx = 0;
                result.ecx = self.processor_feature_01c;
                result.edx = self.processor_feature_01d;
                #[cfg(feature = "sse")]
                {
                    result.ebx |= (CPUID_CLFLUSH_LINE_SIZE as u32) << 8;
                    result.edx |= CPUID_SIMD_01D;
                }
            }
            0x8000_0000 => {
                result.eax = 0x8000_0004;
                result.ebx = 0;
                result.ecx = 0;
                result.edx = 0;
            }
            0x8000_0001 => {
                result.eax = self.processor_version;
                result.ebx = 0;
                result.ecx = self.processor_feature_81c;
                result.edx = self.processor_feature_81d;
            }
            0x8000_0002 => {
                result.eax = u32::from_le_bytes(self.brand_string[0..4].try_into().unwrap());
                result.ebx = u32::from_le_bytes(self.brand_string[4..8].try_into().unwrap());
                result.ecx = u32::from_le_bytes(self.brand_string[8..12].try_into().unwrap());
                result.edx = u32::from_le_bytes(self.brand_string[12..16].try_into().unwrap());
            }
            0x8000_0003 => {
                result.eax = u32::from_le_bytes(self.brand_string[16..20].try_into().unwrap());
                result.ebx = u32::from_le_bytes(self.brand_string[20..24].try_into().unwrap());
                result.ecx = u32::from_le_bytes(self.brand_string[24..28].try_into().unwrap());
                result.edx = u32::from_le_bytes(self.brand_string[28..32].try_into().unwrap());
            }
            0x8000_0004 => {
                result.eax = u32::from_le_bytes(self.brand_string[32..36].try_into().unwrap());
                result.ebx = u32::from_le_bytes(self.brand_string[36..40].try_into().unwrap());
                result.ecx = u32::from_le_bytes(self.brand_string[40..44].try_into().unwrap());
                result.edx = u32::from_le_bytes(self.brand_string[44..48].try_into().unwrap());
            }
            _ => {}
        }

        result
    }
}
