//! Seven-byte secondary dispatch attributes including source EIP; no FP values in the trace.
use crate::cpu::RtReg;
use crate::prelude::SrIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum FpuOp {
    Unsupported,
    Wait,
    Init,
    Clear,
    Nop,
    Load,
    Store,
    LoadControl,
    StoreControl,
    StoreStatus,
    StoreStatusAx,
    LoadRegister,
    StoreRegister,
    Exchange,
    Free,
    IncrementTop,
    DecrementTop,
    ArithmeticRegister,
    ArithmeticMemory,
    CompareRegister,
    CompareMemory,
    Sign,
    Abs,
    Test,
    Examine,
    Zero,
    One,
    Sqrt,
    Trig,
    Atan,
    Scale,
    Extract,
    Remainder,
    Exp2m1,
    LogMultiply,
    Round,
    Constant,
    ConditionalMove,
    LoadEnvironment,
    StoreEnvironment,
    Save,
    Restore,
    /// MMX/SSE/SSE2 form (`sse`). The six bytes after `op` hold a
    /// [`SimdOp`](ir86::prelude::SimdOp), the operands and imm8, not x87
    /// attributes (see [`FpuMinorUop::simd_register`]).
    #[cfg(feature = "sse")]
    Simd,
}

// Format encoded in the low three bits of load/store attributes.
pub(crate) const F32: u8 = 0;
pub(crate) const F64: u8 = 1;
pub(crate) const F80: u8 = 2;
pub(crate) const I16: u8 = 3;
pub(crate) const I32: u8 = 4;
pub(crate) const I64: u8 = 5;
pub(crate) const BCD: u8 = 6;
pub(crate) const POP: u8 = 8;
pub(crate) const TRUNCATE: u8 = 16;

#[derive(Debug, Clone, Copy)]
pub struct FpuMinorUop {
    pub(crate) op: FpuOp,
    // Load/store: format[2:0], pop[3]. Register arithmetic: kind[2:0],
    // ST index[5:3], pop[6], destination-is-STi[7]. Memory arithmetic:
    // kind[2:0], format[5:3]. Register compare: index[2:0], pop-count[4:3],
    // quiet[5], integer-flags[6]. Remaining register ops use index[2:0].
    pub(crate) arg: u8,
    // Four register bits plus three segment bits leave room for the source EIP
    // while keeping the containing Uop at eight bytes.
    access: u8,
    source: [u8; 4],
}

impl FpuMinorUop {
    pub(crate) const fn new(op: FpuOp, arg: u8) -> Self {
        Self {
            op,
            arg,
            access: RtReg::Zero as u8 | ((SrIndex::DS as u8) << 4),
            source: [0; 4],
        }
    }
    pub(crate) fn at(mut self, eip: u32) -> Self {
        self.source = eip.to_le_bytes();
        self
    }
    pub(crate) fn source_eip(self) -> u32 {
        u32::from_le_bytes(self.source)
    }
    /// SystemCpu keeps the 11-bit FOP in the source slot instead of the EIP,
    /// which it finds through its trace map.
    #[allow(dead_code)]
    pub(crate) fn with_fop(self, fop: u16) -> Self {
        self.at(u32::from(fop))
    }
    #[allow(dead_code)]
    pub(crate) fn fop(self) -> u16 {
        self.source_eip() as u16
    }
    pub(crate) fn address(self) -> RtReg {
        match self.access & 15 {
            0 => RtReg::EAX,
            1 => RtReg::ECX,
            2 => RtReg::EDX,
            3 => RtReg::EBX,
            4 => RtReg::ESP,
            5 => RtReg::EBP,
            6 => RtReg::ESI,
            7 => RtReg::EDI,
            8 => RtReg::EIP,
            9 => RtReg::MemAddr,
            10 => RtReg::MemData,
            _ => RtReg::Zero,
        }
    }
    pub(crate) fn segment(self) -> SrIndex {
        SrIndex::from_u8(self.access >> 4).expect("encoded segment")
    }
    pub(crate) fn records_instruction(self) -> bool {
        use FpuOp::*;
        !matches!(
            self.op,
            Unsupported
                | Wait
                | Init
                | Clear
                | Nop
                | LoadControl
                | StoreControl
                | StoreStatus
                | StoreStatusAx
                | LoadEnvironment
                | StoreEnvironment
                | Save
                | Restore
        ) && !self.is_simd()
    }
    #[inline]
    pub(crate) fn is_simd(self) -> bool {
        #[cfg(feature = "sse")]
        return self.op == FpuOp::Simd;
        #[cfg(not(feature = "sse"))]
        false
    }
    pub(crate) fn accesses_data(self) -> bool {
        matches!(
            self.op,
            FpuOp::Load | FpuOp::Store | FpuOp::ArithmeticMemory | FpuOp::CompareMemory
        )
    }
    pub(crate) fn register(opcode: u8, reg: u8, index: u8) -> Self {
        use FpuOp::*;
        let byte = 0xc0 | (reg << 3) | index;
        let (op, arg) = match (opcode, reg, byte) {
            (0xd8, 2 | 3, _) => (CompareRegister, index | if reg == 3 { POP } else { 0 }),
            (0xd8, _, _) => (ArithmeticRegister, reg | (index << 3)),
            (0xdc, 0 | 1 | 4..=7, _) => {
                let kind = match reg {
                    4..=7 => reg ^ 1,
                    _ => reg,
                };
                (ArithmeticRegister, kind | (index << 3) | 0x80)
            }
            (0xde, _, 0xd9) => (CompareRegister, 1 | (2 << 3)),
            (0xde, 0 | 1 | 4..=7, _) => {
                let kind = match reg {
                    4..=7 => reg ^ 1,
                    _ => reg,
                };
                (ArithmeticRegister, kind | (index << 3) | 0xc0)
            }
            (0xda | 0xdb, 0..=3, _) => (
                ConditionalMove,
                index | (reg << 3) | if opcode == 0xdb { 0x20 } else { 0 },
            ),
            (0xda, _, 0xe9) => (CompareRegister, 1 | (2 << 3) | 0x20),
            (0xd9, 0, _) => (LoadRegister, index),
            (0xd9, 1, _) => (Exchange, index),
            (0xd9, _, 0xd0) => (Nop, 0),
            (0xd9, _, 0xe0) => (Sign, 0),
            (0xd9, _, 0xe1) => (Abs, 0),
            (0xd9, _, 0xe4) => (Test, 0),
            (0xd9, _, 0xe5) => (Examine, 0),
            (0xd9, _, 0xe8) => (One, 0),
            (0xd9, _, 0xee) => (Zero, 0),
            (0xd9, _, 0xe9..=0xed) => (Constant, byte - 0xe9),
            (0xd9, _, 0xf6) => (DecrementTop, 0),
            (0xd9, _, 0xf7) => (IncrementTop, 0),
            (0xd9, _, 0xfa) => (Sqrt, 0),
            (0xd9, _, 0xfe) => (Trig, 0), // sin
            (0xd9, _, 0xff) => (Trig, 1), // cos
            (0xd9, _, 0xfb) => (Trig, 2), // sin, cos
            (0xd9, _, 0xf2) => (Trig, 3), // tan, 1
            (0xd9, _, 0xf3) => (Atan, 0),
            (0xd9, _, 0xfc) => (Round, 0),
            (0xd9, _, 0xfd) => (Scale, 0),
            (0xd9, _, 0xf4) => (Extract, 0),
            (0xd9, _, 0xf8) => (Remainder, 0),
            (0xd9, _, 0xf5) => (Remainder, 1),
            (0xd9, _, 0xf0) => (Exp2m1, 0),
            (0xd9, _, 0xf1) => (LogMultiply, 0),
            (0xd9, _, 0xf9) => (LogMultiply, 1),
            (0xdd, 0, _) => (Free, index),
            (0xdd, 2 | 3, _) => (StoreRegister, index | if reg == 3 { POP } else { 0 }),
            (0xdd, 4 | 5, _) => (
                CompareRegister,
                index | 0x20 | if reg == 5 { POP } else { 0 },
            ),
            (0xdb, _, 0xe2) => (Clear, 0),
            (0xdb, _, 0xe3) => (Init, 0),
            (0xdb | 0xdf, 5 | 6, _) => (
                CompareRegister,
                index
                    | 0x40
                    | if reg == 5 { 0x20 } else { 0 }
                    | if opcode == 0xdf { POP } else { 0 },
            ),
            (0xdf, _, 0xe0) => (StoreStatusAx, 0),
            _ => (Unsupported, byte),
        };
        Self::new(op, arg)
    }
    pub(crate) fn memory(
        opcode: u8,
        reg: u8,
        address: RtReg,
        segment: SrIndex,
        is_opsize32: bool,
    ) -> Self {
        use FpuOp::*;
        let (op, arg) = match (opcode, reg) {
            (0xd8 | 0xda | 0xdc | 0xde, _) => {
                let format = match opcode {
                    0xd8 => F32,
                    0xda => I32,
                    0xdc => F64,
                    _ => I16,
                };
                if matches!(reg, 2 | 3) {
                    (CompareMemory, format | if reg == 3 { POP } else { 0 })
                } else {
                    (ArithmeticMemory, reg | (format << 3))
                }
            }
            (0xd9, 0) => (Load, F32),
            (0xdd, 0) => (Load, F64),
            (0xdb, 5) => (Load, F80),
            (0xdb, 0) => (Load, I32),
            (0xdf, 0) => (Load, I16),
            (0xdf, 5) => (Load, I64),
            (0xdf, 4) => (Load, BCD),
            (0xdf, 6) => (Store, BCD | POP),
            (0xd9, 2 | 3) => (Store, F32 | if reg == 3 { POP } else { 0 }),
            (0xdd, 2 | 3) => (Store, F64 | if reg == 3 { POP } else { 0 }),
            (0xdb, 7) => (Store, F80 | POP),
            (0xdb, 1) => (Store, I32 | POP | TRUNCATE),
            (0xdd, 1) => (Store, I64 | POP | TRUNCATE),
            (0xdf, 1) => (Store, I16 | POP | TRUNCATE),
            (0xdb, 2 | 3) => (Store, I32 | if reg == 3 { POP } else { 0 }),
            (0xdf, 2 | 3) => (Store, I16 | if reg == 3 { POP } else { 0 }),
            (0xdf, 7) => (Store, I64 | POP),
            (0xd9, 4) => (LoadEnvironment, u8::from(is_opsize32)),
            (0xd9, 6) => (StoreEnvironment, u8::from(is_opsize32)),
            (0xdd, 4) => (Restore, u8::from(is_opsize32)),
            (0xdd, 6) => (Save, u8::from(is_opsize32)),
            (0xd9, 5) => (LoadControl, 0),
            (0xd9, 7) => (StoreControl, 0),
            (0xdd, 7) => (StoreStatus, 0),
            _ => (Unsupported, reg),
        };
        Self {
            op,
            arg,
            access: address as u8 | ((segment as u8) << 4),
            source: [0; 4],
        }
    }
}

/// MMX/SSE/SSE2 forms share the x87 Uop so that no engine gains a dispatcher
/// variant. `arg` is [`SIMD_MEMORY`] for a memory form, or [`SIMD_IMPLICIT`]
/// (plus 4 for 32-bit addressing) for MASKMOVQ/MASKMOVDQU; `access` holds
/// the address register (EDI for the implicit operand) and segment as for
/// x87; `source` holds the `SimdOp` (little-endian), ModR/M.reg in bits 2:0
/// and the r/m register in bits 5:3 of the third byte, and the imm8.
#[cfg(feature = "sse")]
impl FpuMinorUop {
    pub(crate) const SIMD_MEMORY: u8 = 1;
    pub(crate) const SIMD_IMPLICIT: u8 = 2;

    pub(crate) fn simd_register(op: ir86::prelude::SimdOp, reg: u8, rm: u8, imm: u8) -> Self {
        let [lo, hi] = (op as u16).to_le_bytes();
        Self {
            source: [lo, hi, (reg & 7) | ((rm & 7) << 3), imm],
            ..Self::new(FpuOp::Simd, 0)
        }
    }

    pub(crate) fn simd_memory(
        op: ir86::prelude::SimdOp,
        reg: u8,
        address: RtReg,
        segment: SrIndex,
        imm: u8,
    ) -> Self {
        let [lo, hi] = (op as u16).to_le_bytes();
        Self {
            op: FpuOp::Simd,
            arg: Self::SIMD_MEMORY,
            access: address as u8 | ((segment as u8) << 4),
            source: [lo, hi, reg & 7, imm],
        }
    }

    /// MASKMOVQ/MASKMOVDQU: registers plus the implicit `segment:[(E)DI]`.
    pub(crate) fn simd_implicit(
        op: ir86::prelude::SimdOp,
        reg: u8,
        rm: u8,
        segment: SrIndex,
        is_addr32: bool,
    ) -> Self {
        let [lo, hi] = (op as u16).to_le_bytes();
        Self {
            op: FpuOp::Simd,
            arg: Self::SIMD_IMPLICIT | if is_addr32 { 4 } else { 0 },
            access: RtReg::EDI as u8 | ((segment as u8) << 4),
            source: [lo, hi, (reg & 7) | ((rm & 7) << 3), 0],
        }
    }

    /// The Uop of an MMX/SSE/SSE2 IR form, shared by the three tracers.
    /// `lea` emits the address computation of a memory operand and returns
    /// its register and segment, or `None` when the engine does not lower
    /// that operand (16-bit addressing on HarvardUME and FlatUME); the form
    /// then has no Uop either.
    pub(crate) fn simd_lowering(
        ir: ir86::prelude::IrOp,
        lea: impl FnOnce(ir86::prelude::MemOpr) -> Option<(RtReg, SrIndex)>,
    ) -> Option<Self> {
        use ir86::prelude::IrOp;
        Some(match ir {
            IrOp::SIMD(op) => Self::simd_register(op, 0, 0, 0),
            IrOp::SIMD_R(op, reg, rm, imm) => Self::simd_register(op, reg.index(), rm.index(), imm),
            IrOp::SIMD_S(op, reg, rm, di) => {
                Self::simd_implicit(op, reg.index(), rm.index(), di.segment, di.is_addr32)
            }
            IrOp::SIMD_M(op, reg, ma, imm) => {
                let (address, segment) = lea(ma)?;
                Self::simd_memory(op, reg.index(), address, segment, imm)
            }
            _ => unreachable!("not an MMX/SSE/SSE2 form"),
        })
    }

    /// For the implicit `[(E)DI]` operand: whether the address size is 32.
    pub(crate) fn simd_implicit_addr32(self) -> Option<bool> {
        (self.arg & Self::SIMD_IMPLICIT != 0).then_some(self.arg & 4 != 0)
    }

    pub(crate) fn simd_op(self) -> ir86::prelude::SimdOp {
        ir86::prelude::SimdOp::from_u16(u16::from_le_bytes([self.source[0], self.source[1]]))
            .expect("encoded SimdOp")
    }

    pub(crate) fn simd_is_memory(self) -> bool {
        self.arg == Self::SIMD_MEMORY
    }

    /// ModR/M.reg register number.
    pub(crate) fn simd_reg(self) -> usize {
        usize::from(self.source[2] & 7)
    }

    /// ModR/M.r/m register number of a register form.
    pub(crate) fn simd_rm(self) -> usize {
        usize::from((self.source[2] >> 3) & 7)
    }

    pub(crate) fn simd_imm(self) -> u8 {
        self.source[3]
    }
}

const _: () = assert!(core::mem::size_of::<FpuMinorUop>() == 7);
