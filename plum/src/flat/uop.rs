//! Intermediate Micro Opcode

use ir86::encoding::{CC, SrIndex};
use ir86::types::Offset32;

use crate::cpu::{RtReg, RtReg8};
use crate::flat::sib::SibIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FuncIndex(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AddrIndex(pub u32);

impl AddrIndex {
    pub const BAD_ADDRESS: Self = AddrIndex(u32::MAX);
}

/// Bit-test operation; CF always receives the original selected bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitAction {
    Test,
    Set,
    Reset,
    Complement,
}

#[derive(Debug, Clone, Copy)]
pub enum LoopKind {
    Count,
    WhileZero,
    WhileNotZero,
    CountZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoKind {
    FpuDisabled,
    FpuAddress16,
    /// An MMX/SSE/SSE2 memory operand with 16-bit addressing.
    SimdAddress16,
    /// FS/GS override other than a GS-relative `disp32` load or indirect call.
    SegmentOverride,
    /// MOV Sreg from memory, PUSH/POP Sreg.
    SegmentMemory,
}

/// Valid forms UME deliberately does not implement and has no plan to.
/// Unlike `TodoKind`, these are not pending work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedKind {
    /// ENTER with a nonzero nesting level.
    NestedEnter,
    /// An IR form FlatUME has no lowering for: 16-bit branches, far
    /// transfers, privileged instructions and the like. Reported apart from
    /// #UD ([`Exception::InvalidOpcode`](crate::ume::Exception::InvalidOpcode)),
    /// which is only for encodings the emulated CPU does not define.
    NoLowering,
    /// A direct near call beyond the 65536-entry call-site table.
    CallSites,
}

/// Attributes only; addresses and count are read from guest registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringKind {
    Movs,
    Stos,
    Lods,
    Cmps,
    Scas,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringWidth {
    Byte,
    Word,
    Dword,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringRepeat {
    Once,
    Count,
    WhileZero,
    WhileNotZero,
}

/// Extension applied to a single memory read; the destination is always 32-bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadExtension {
    Zero,
    Sign,
}

/// Compact string attributes. Only the typed constructor can create a value.
/// Packing avoids wide payload extraction in the common Wasm dispatcher.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StringOp(u16);

impl StringOp {
    pub const fn new(
        kind: StringKind,
        width: StringWidth,
        repeat: StringRepeat,
        segment: SrIndex,
        addr32: bool,
    ) -> Self {
        Self(
            (kind as u16)
                | ((width as u16) << 3)
                | ((repeat as u16) << 5)
                | ((segment as u16) << 7)
                | ((addr32 as u16) << 10),
        )
    }
    pub const fn kind(self) -> StringKind {
        match self.0 & 7 {
            0 => StringKind::Movs,
            1 => StringKind::Stos,
            2 => StringKind::Lods,
            3 => StringKind::Cmps,
            4 => StringKind::Scas,
            _ => unreachable!(),
        }
    }
    pub const fn width(self) -> StringWidth {
        match (self.0 >> 3) & 3 {
            0 => StringWidth::Byte,
            1 => StringWidth::Word,
            2 => StringWidth::Dword,
            _ => unreachable!(),
        }
    }
    pub const fn repeat(self) -> StringRepeat {
        match (self.0 >> 5) & 3 {
            0 => StringRepeat::Once,
            1 => StringRepeat::Count,
            2 => StringRepeat::WhileZero,
            _ => StringRepeat::WhileNotZero,
        }
    }
    pub const fn segment(self) -> SrIndex {
        match SrIndex::from_u8(((self.0 >> 7) & 7) as u8) {
            Some(segment) => segment,
            None => unreachable!(),
        }
    }
    pub const fn is_addr32(self) -> bool {
        self.0 & (1 << 10) != 0
    }
}

impl core::fmt::Debug for StringOp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StringOp")
            .field("kind", &self.kind())
            .field("width", &self.width())
            .field("repeat", &self.repeat())
            .field("segment", &self.segment())
            .field("is_addr32", &self.is_addr32())
            .finish()
    }
}

#[cfg(feature = "flag-elision")]
#[derive(Debug, Clone, Copy)]
pub enum ValueShift {
    Shl,
    Shr,
    Sar,
}

/// Supplemental binary updates share width-specific execution helpers.
#[derive(Debug, Clone, Copy)]
pub enum BinaryRmwKind {
    Adc,
    Add,
    And,
    Or,
    Sbb,
    Sub,
    Xor,
}

/// Operation and width for memory shifts. A typed one-byte attribute keeps the
/// primary Uop below 256 variants; the secondary dispatch is outside the CPU loop.
#[derive(Debug, Clone, Copy)]
pub enum ShiftRmwOp {
    Shl,
    Shl16,
    Shl8,
    Shr,
    Shr16,
    Shr8,
    Sar,
    Sar16,
    Sar8,
    Rol,
    Rol16,
    Rol8,
    Ror,
    Ror16,
    Ror8,
}

/// Width-specific unary updates share one primary dispatch entry.
#[derive(Debug, Clone, Copy)]
pub enum UnaryRmwOp {
    Inc,
    Inc16,
    Inc8,
    Dec,
    Dec16,
    Dec8,
    Neg,
    Neg16,
    Neg8,
    Not,
    Not16,
    Not8,
}

// RMW operations carry an address register; immediate forms avoid scratch loads.
// Existing fast binary variants remain dedicated. Supplemental binary, unary, and
// shift attributes dispatch in separate helpers to keep the primary table compact.
// Ordinary integer operations Cdq / MulR / BswapR belong in the primary dispatcher.
// Rare Loop / PopMemR / PopMemSIB also remain primary: nesting their 32-bit
// target/disp in Minor would enlarge the outer Uop to 12 bytes on native and Wasm.
/// Micro Operation Code
#[derive(Debug, Clone, Copy)]
pub enum Uop {
    #[cfg(feature = "flag-elision")]
    ValueAddR(RtReg, RtReg),
    #[cfg(feature = "flag-elision")]
    ValueXorR(RtReg, RtReg),
    #[cfg(feature = "flag-elision")]
    ValueAndI(RtReg, u32),
    #[cfg(feature = "flag-elision")]
    ValueAddI(RtReg, u32),
    #[cfg(feature = "flag-elision")]
    ValueSubI(RtReg, u32),
    #[cfg(feature = "flag-elision")]
    ValueSubR(RtReg, RtReg),
    #[cfg(feature = "flag-elision")]
    ValueAndR(RtReg, RtReg),
    #[cfg(feature = "flag-elision")]
    ValueOrR(RtReg, RtReg),
    #[cfg(feature = "flag-elision")]
    ValueOrI(RtReg, u32),
    #[cfg(feature = "flag-elision")]
    ValueXorI(RtReg, u32),
    #[cfg(feature = "flag-elision")]
    ValueReg(BinaryRmwKind, RtReg, RtReg),
    #[cfg(feature = "flag-elision")]
    ValueImm(BinaryRmwKind, RtReg, u32),
    #[cfg(feature = "flag-elision")]
    ValueNeg(RtReg),
    #[cfg(feature = "flag-elision")]
    ValueShift(ValueShift, RtReg, Option<u8>),
    #[cfg(feature = "flag-elision")]
    ValueMulR(RtReg, RtReg),
    #[cfg(feature = "flag-elision")]
    ValueMulRI(RtReg, RtReg, i32),
    AdcI(RtReg, u32),
    AdcI16(RtReg, u16),
    AdcI8(RtReg8, u8),

    AdcR(RtReg, RtReg),
    AdcR16(RtReg, RtReg),
    AdcR8(RtReg8, RtReg8),

    // Dword register destination and a memory source at the address in the second register.
    AddLoad(RtReg, RtReg),
    SubLoad(RtReg, RtReg),
    AndLoad(RtReg, RtReg),
    OrLoad(RtReg, RtReg),
    XorLoad(RtReg, RtReg),
    // CMP and TEST only write flags. Operands follow x86 order; memory holds an address register.
    CmpRegMem(RtReg, RtReg),
    CmpMemReg(RtReg, RtReg),
    TestMemReg(RtReg, RtReg),
    /// Add Register and Immediate Value
    AddI(RtReg, u32),
    /// Add Register and Immediate Value (16-bit)
    AddI16(RtReg, u16),
    // Add Register and Immediate Value (8-bit)
    AddI8(RtReg8, u8),

    /// Add Register Values
    AddR(RtReg, RtReg),
    /// Add Register Values (16-bit)
    AddR16(RtReg, RtReg),
    /// Add Register Values (8-bit)
    AddR8(RtReg8, RtReg8),
    /// Add Register Value and Memory Value (Read-Modify-Write)
    AddRMW(RtReg, RtReg),

    /// Add Register Value and Memory Value (Read-Modify-Write) (8-bit)
    AddRMW8(RtReg, RtReg8),

    /// And Register and Immediate Value
    AndI(RtReg, u32),
    /// And Register and Immediate Value (16-bit)
    AndI16(RtReg, u16),
    /// And Register and Immediate Value (8-bit)
    AndI8(RtReg8, u8),

    /// And Register Values
    AndR(RtReg, RtReg),
    /// And Register Values (16-bit)
    AndR16(RtReg, RtReg),
    /// And Register Values (8-bit)
    AndR8(RtReg8, RtReg8),
    /// And Register Value and Memory Value (Read-Modify-Write)
    AndRMW(RtReg, RtReg),
    BinaryIRMW(BinaryRmwKind, RtReg, u32),
    BinaryIRMW16(BinaryRmwKind, RtReg, u16),
    BinaryIRMW8(BinaryRmwKind, RtReg, u8),

    BinaryRMW(BinaryRmwKind, RtReg, RtReg),
    BinaryRMW16(BinaryRmwKind, RtReg, RtReg),
    BinaryRMW8(BinaryRmwKind, RtReg, RtReg8),

    /// Adjust MemAddr using a signed bit-string index.
    BitAddr(RtReg),
    BitAddr16(RtReg),
    BitI(BitAction, RtReg, u8),
    BitI16(BitAction, RtReg, u8),

    /// Memory bit update; action must be Set, Reset, or Complement.
    BitIRMW(BitAction, RtReg, u8),
    /// Memory bit update; action must be Set, Reset, or Complement.
    BitIRMW16(BitAction, RtReg, u8),

    BitR(BitAction, RtReg, RtReg),
    BitR16(BitAction, RtReg, RtReg),
    /// Memory bit update; action must be Set, Reset, or Complement.
    BitRRMW(BitAction, RtReg, RtReg),
    /// Memory bit update; action must be Set, Reset, or Complement.
    BitRRMW16(BitAction, RtReg, RtReg),
    BsfR(RtReg, RtReg),
    BsfR16(RtReg, RtReg),
    BsrR(RtReg, RtReg),
    BsrR16(RtReg, RtReg),
    /// Reverse register bytes, preserving flags.
    BswapR(RtReg),

    /// Call to Function
    Call(FuncIndex, Offset32),
    /// Call to Register
    CallR(RtReg, Offset32),

    /// Sign-extend EAX into EDX:EAX.
    Cdq,
    Cwd,

    /// Conditional register move (source memory is loaded separately).
    Cmov(CC, RtReg, RtReg),
    /// Conditional register move, preserving the destination's upper 16 bits.
    Cmov16(CC, RtReg, RtReg),

    /// Compare Register and Immediate Value
    CmpI(RtReg, u32),
    /// Compare Register and Immediate Value (16-bit)
    CmpI16(RtReg, u16),
    /// Compare Register and Immediate Value (8-bit)
    CmpI8(RtReg8, u8),
    /// Compare Register Values
    CmpR(RtReg, RtReg),
    /// Compare Register Values (16-bit)
    CmpR16(RtReg, RtReg),
    /// Compare Register Values (8-bit)
    CmpR8(RtReg8, RtReg8),
    /// Decrement Register
    DecR(RtReg),
    DecR16(RtReg),
    DecR8(RtReg8),

    /// Divide by Register
    DivR(RtReg),
    DivR16(RtReg),
    DivR8(RtReg8),

    /// Unresolved Fetch Next Instruction
    FetchNext(Offset32),
    /// Integer Divide by Register
    IDivR(RtReg),
    IDivR16(RtReg),
    IDivR8(RtReg8),
    IMulFullR(RtReg),
    IMulFullR16(RtReg),
    IMulFullR8(RtReg8),

    /// Integer Multiply by Register
    IMulR(RtReg, RtReg),
    IMulR16(RtReg, RtReg),
    /// Integer Multiply by Register and Immediate Value
    IMulRI(RtReg, RtReg, i32),
    IMulRI16(RtReg, RtReg, i16),

    /// Increment Register
    IncR(RtReg),
    IncR16(RtReg),
    IncR8(RtReg8),

    /// Conditional Jump (Resolved)
    Jcc(CC, AddrIndex),

    /// Conditional Jump (Unresolved)
    JccU(CC, Offset32),
    /// Jump if Not Zero
    Jnz(AddrIndex),

    /// Unconditional Jump
    Jump(AddrIndex),
    /// Unconditional Jump to Register
    JumpR(RtReg),

    /// Jump if Zero
    Jz(AddrIndex),

    /// Load Effective Address (LEA) with Base and Displacement
    LeaBD(RtReg, RtReg, Offset32),
    /// Load Effective Address (LEA) with Base, Index, Scale and Displacement
    LeaSIB(RtReg, SibIndex, Offset32),
    /// Load from Memory with Base and Displacement
    LoadBD32(RtReg, RtReg, Offset32),

    /// Load Constant
    LoadConst(RtReg, u32),
    /// Load Constant 16-bit
    LoadConst16(RtReg, u16),
    /// Load Constant 8-bit
    LoadConst8(RtReg8, u8),

    /// Load and extend into a full 32-bit destination, preserving flags.
    LoadExtend16(LoadExtension, RtReg, RtReg),
    LoadExtend8(LoadExtension, RtReg, RtReg),

    /// Load from Memory with Base Register (16-bit)
    LoadR16(RtReg, RtReg),

    /// Load from Memory with Base Register
    LoadR32(RtReg, RtReg),

    /// Load from Memory with Base Register (8-bit)
    LoadR8(RtReg8, RtReg),
    /// Load from Memory with Base, Index, Scale and Displacement
    LoadSIB32(RtReg, SibIndex, Offset32),

    Loop(LoopKind, bool, Offset32),
    /// Move with Sign-Extend 16-bit to 32-bit
    MovSx16(RtReg, RtReg),
    /// Move with Sign-Extend 8-bit to 32-bit
    MovSx8(RtReg, RtReg8),
    /// Move with Zero-Extend 16-bit to 32-bit
    MovZx16(RtReg, RtReg),

    /// Move with Zero-Extend 8-bit to 32-bit
    MovZx8(RtReg, RtReg8),

    /// Move Register
    Move(RtReg, RtReg),
    /// Move Register 16-bit
    Move16(RtReg, RtReg),
    /// Move Register 8-bit
    Move8(RtReg8, RtReg8),
    MulR(RtReg),
    MulR16(RtReg),
    MulR8(RtReg8),

    /// Negate Register Value
    NegR(RtReg),
    NegR16(RtReg),
    NegR8(RtReg8),

    /// No operation (consumes an execution step).
    Nop,

    /// Not Register
    NotR(RtReg),

    /// Or Register and Immediate Value
    OrI(RtReg, u32),
    /// Or Register and Immediate Value (16-bit)
    OrI16(RtReg, u16),
    /// Or Register and Immediate Value (8-bit)
    OrI8(RtReg8, u8),

    /// Or Register Values
    OrR(RtReg, RtReg),
    /// Or Register Values (16-bit)
    OrR16(RtReg, RtReg),
    /// Or Register Values (8-bit)
    OrR8(RtReg8, RtReg8),
    /// Or Register Value and Memory Value (Read-Modify-Write)
    OrRMW(RtReg, RtReg),

    /// POP memory computes its destination using the incremented ESP.
    PopMemR(RtReg, Offset32),
    PopMemSIB(SibIndex, Offset32),
    /// Pop into Register
    PopR(RtReg),
    Popcnt(RtReg, RtReg),
    Popcnt16(RtReg, RtReg),

    /// Push Immediate Value
    PushI(u32),
    /// Push Register Value
    PushR(RtReg),
    /// Return from Function
    Ret(u16),
    RolI(RtReg, u8),
    RolI16(RtReg, u8),
    RolI8(RtReg8, u8),

    RolIRMW(RtReg, u8),

    RolRCl(RtReg),
    RolRCl16(RtReg),
    RolRCl8(RtReg8),

    RorI(RtReg, u8),
    RorI16(RtReg, u8),
    RorI8(RtReg8, u8),

    RorRCl(RtReg),
    RorRCl16(RtReg),
    RorRCl8(RtReg8),

    /// Shift Arithmetic Right Register by Immediate Value
    SarI(RtReg, u8),
    /// Shift Arithmetic Right Register by Immediate Value (16-bit)
    SarI16(RtReg, u8),
    /// Shift Arithmetic Right Register by Immediate Value (8-bit)
    SarI8(RtReg8, u8),

    /// Shift Arithmetic Right Register by Cl
    SarRCl(RtReg),
    /// Shift Arithmetic Right Register by Cl (16-bit)
    SarRCl16(RtReg),
    /// Shift Arithmetic Right Register by Cl (8-bit)
    SarRCl8(RtReg8),

    SbbI(RtReg, u32),
    SbbI16(RtReg, u16),
    SbbI8(RtReg8, u8),

    SbbR(RtReg, RtReg),
    SbbR16(RtReg, RtReg),
    SbbR8(RtReg8, RtReg8),

    /// Set Value of Register Based on Condition Code
    SetCC(CC, RtReg8),
    ShiftIRMW(ShiftRmwOp, RtReg, u8),

    ShiftRMWCl(ShiftRmwOp, RtReg),

    /// Shift Left Register by Immediate Value
    ShlI(RtReg, u8),
    /// Shift Left Register by Immediate Value (16-bit)
    ShlI16(RtReg, u8),
    /// Shift Left Register by Immediate Value (8-bit)
    ShlI8(RtReg8, u8),

    /// Shift Left Register by Cl
    ShlRCl(RtReg),
    /// Shift Left Register by Cl (16-bit)
    ShlRCl16(RtReg),
    /// Shift Left Register by Cl (8-bit)
    ShlRCl8(RtReg8),

    ShldI(RtReg, RtReg, u8),
    ShldI16(RtReg, RtReg, u8),

    ShldRCl(RtReg, RtReg),
    ShldRCl16(RtReg, RtReg),

    /// Shift Right Register by Immediate Value
    ShrI(RtReg, u8),
    /// Shift Right Register by Immediate Value (16-bit)
    ShrI16(RtReg, u8),
    /// Shift Right Register by Immediate Value (8-bit)
    ShrI8(RtReg8, u8),

    /// Shift Right Register by Cl
    ShrRCl(RtReg),
    /// Shift Right Register by Cl (16-bit)
    ShrRCl16(RtReg),
    /// Shift Right Register by Cl (8-bit)
    ShrRCl8(RtReg8),

    ShrdI(RtReg, RtReg, u8),
    ShrdI16(RtReg, RtReg, u8),

    ShrdRCl(RtReg, RtReg),
    ShrdRCl16(RtReg, RtReg),

    /// Store to Memory with Base and Displacement
    StoreBD32(RtReg, RtReg, Offset32),
    /// Store to Memory with Base Register (16-bit)
    StoreR16(RtReg, RtReg),

    /// Store to Memory with Base Register
    StoreR32(RtReg, RtReg),
    /// Store to Memory with Base Register (8-bit)
    StoreR8(RtReg8, RtReg),
    /// Store to Memory with Base, Index, Scale and Displacement
    StoreSIB32(RtReg, SibIndex, Offset32),
    String(StringOp),

    /// Subtract Register and Immediate Value
    SubI(RtReg, u32),
    /// Subtract Register and Immediate Value (16-bit)
    SubI16(RtReg, u16),
    /// Subtract Register and Immediate Value (8-bit)
    SubI8(RtReg8, u8),

    /// Subtract Register Values
    SubR(RtReg, RtReg),
    /// Subtract Register Values (16-bit)
    SubR16(RtReg, RtReg),
    /// Subtract Register Values (8-bit)
    SubR8(RtReg8, RtReg8),
    /// Subtract Register Value and Memory Value (Read-Modify-Write)
    SubRMW(RtReg, RtReg),

    /// Subtract Register Value and Memory Value (Read-Modify-Write) (8-bit)
    SubRMW8(RtReg, RtReg8),

    /// System Call
    SysCall,
    /// System Enter
    SysEnter,
    /// Software Interrupt
    Swi(u8),

    /// Test Register and Immediate Value
    TestI(RtReg, u32),
    /// Test Register and Immediate Value (16-bit)
    TestI16(RtReg, u16),
    /// Test Register and Immediate Value (8-bit)
    TestI8(RtReg8, u8),
    /// Test Register Values
    TestR(RtReg, RtReg),
    /// Test Register Values (16-bit)
    TestR16(RtReg, RtReg),
    /// Test Register Values (8-bit)
    TestR8(RtReg8, RtReg8),

    UnaryRMW(UnaryRmwOp, RtReg),

    /// Compare AL/AX/EAX with the destination, then exchange or load the accumulator.
    CmpxchgR(RtReg, RtReg),
    CmpxchgR16(RtReg, RtReg),
    CmpxchgR8(RtReg8, RtReg8),
    CmpxchgRMW(RtReg, RtReg),
    CmpxchgRMW16(RtReg, RtReg),
    CmpxchgRMW8(RtReg, RtReg8),

    /// Add register to memory and return the original memory value to the register.
    XaddRMW(RtReg, RtReg),
    XaddRMW16(RtReg, RtReg),
    XaddRMW8(RtReg, RtReg8),

    /// Exchange Register
    XaddR(RtReg, RtReg),
    XaddR16(RtReg, RtReg),
    XaddR8(RtReg8, RtReg8),
    XchgR(RtReg, RtReg),
    XchgR16(RtReg, RtReg),
    XchgR8(RtReg8, RtReg8),

    /// Exchange memory at the first register's address with the second register.
    XchgRMW(RtReg, RtReg),
    XchgRMW16(RtReg, RtReg),
    XchgRMW8(RtReg, RtReg8),

    /// Xor Register and Immediate Value
    XorI(RtReg, u32),
    /// Xor Register and Immediate Value (16-bit)
    XorI16(RtReg, u16),
    /// Xor Register and Immediate Value (8-bit)
    XorI8(RtReg8, u8),

    /// Xor Register Values
    XorR(RtReg, RtReg),
    /// Xor Register Values (16-bit)
    XorR16(RtReg, RtReg),
    /// Xor Register Values (8-bit)
    XorR8(RtReg8, RtReg8),
    /// Xor Register Value and Memory Value (Read-Modify-Write)
    XorRMW(RtReg, RtReg),

    /// Less frequent instructions; classification is documented in the allocation plan.
    Minor(UopMinor),
    #[cfg(feature = "x87")]
    Fpu(crate::fpu::FpuMinorUop),
    /// Unresolved guest jump; patched to Jump within a cache generation.
    JumpU(Offset32),
    /// Commit a complete guest write before fetching potentially changed code.
    CheckWrite(Offset32),
}

impl Uop {
    // Conditional branches retain their sequential successor in the trace.
    // Taken targets resolve separately; a failed speculative decode is deferred
    // to FetchNext, so an untaken successor cannot fault the selected path.
    pub(super) fn ends_trace(&self) -> bool {
        matches!(
            self,
            Self::Call(..)
                | Self::CallR(..)
                | Self::Ret(..)
                | Self::Jump(..)
                | Self::JumpU(..)
                | Self::JumpR(..)
                | Self::Loop(..)
                | Self::Swi(..)
                | Self::Minor(
                    UopMinor::RdTsc
                        | UopMinor::LoadSegment(..)
                        | UopMinor::InvalidOpcode
                        | UopMinor::PortIo
                        | UopMinor::Todo(_)
                        | UopMinor::Unsupported(_)
                )
        )
    }
}

/// Minor Uop for special instructions
#[derive(Debug, Clone, Copy)]
pub enum UopMinor {
    /// Clear Direction Flag
    Cld,
    Std,

    /// Identify the Processor and its Features
    Cpuid,

    /// Read Time-Stamp Counter
    RdTsc,

    Clc,
    Stc,
    Cmc,
    Lahf,
    Sahf,
    PushFd,
    PopFd,
    Xlat,
    Daa,
    Das,
    Aaa,
    Aas,
    Aam(u8),
    Aad(u8),
    /// Save all general-purpose registers.
    PushAd,
    /// Restore all general-purpose registers.
    PopAd,
    RclI(RtReg, u8),
    RclRCl(RtReg),
    RclI8(RtReg8, u8),
    RclRCl8(RtReg8),
    RclI16(RtReg, u8),
    RclRCl16(RtReg),
    RcrI(RtReg, u8),
    RcrRCl(RtReg),
    RcrI8(RtReg8, u8),
    RcrRCl8(RtReg8),
    RcrI16(RtReg, u8),
    RcrRCl16(RtReg),
    Enter(u16),
    /// MOV Sreg, r16: trap to the host with the selector in the low word.
    LoadSegment(SrIndex, RtReg),
    /// MOV r16/r32, Sreg: r32 receives the zero-extended selector.
    ReadSegment(RtReg, SrIndex, bool),
    /// Explicit deferred instruction: return an emulator exception, never panic.
    Todo(TodoKind),
    /// User-mode port I/O is forbidden, even with a zero REP count.
    PortIo,
    InvalidOpcode,
    /// LODS only; MOVS/STOS/CMPS/SCAS stay in the primary String variant.
    String(StringOp),
    // Fused carry rotates stay Minor; existing Minor tags remain unchanged.
    /// Valid form UME does not implement: return an emulator exception, never panic.
    Unsupported(UnsupportedKind),
    /// Read the frame before committing either ESP or EBP.
    Leave,
}

// A trace entry must remain compact on both native and Wasm targets.
const _: () = assert!(core::mem::size_of::<LoadExtension>() == 1);
const _: () = assert!(core::mem::size_of::<ShiftRmwOp>() == 1);
const _: () = assert!(core::mem::size_of::<UnaryRmwOp>() == 1);
const _: () = assert!(core::mem::size_of::<Uop>() <= 8);
