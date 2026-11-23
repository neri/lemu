//! An x86 instruction decoder
use crate::_prelude_::*;
use crate::simd::{SimdPrefix, simd_lookup, simd_shape};

/// An x86 instruction decoder
pub struct Decoder {
    use32: bool,
}

// LOCK is ignored; only semantically used F2/F3 prefixes need decode state.
#[derive(Clone, Copy)]
enum RepeatPrefix {
    None,
    F2,
    F3,
}

struct DecoderContext {
    segment_override: Option<SrIndex>,
    is_opsize32: bool,
    is_addr32: bool,
    repeat_prefix: RepeatPrefix,
}

impl Decoder {
    /// Creates a new instance of the decoder.
    #[inline]
    pub const fn new(use32: bool) -> Self {
        Self { use32 }
    }

    /// Creates a new instance of the decoder that decodes instructions in 32-bit mode by default.
    #[inline]
    pub const fn use32() -> Self {
        Self::new(true)
    }

    /// Creates a new instance of the decoder that decodes instructions in 16-bit mode by default.
    #[inline]
    pub const fn use16() -> Self {
        Self::new(false)
    }

    /// Sets whether the decoder should decode instructions in 32-bit mode by default.
    #[inline]
    pub fn set_use32(&mut self, use32: bool) {
        self.use32 = use32;
    }

    /// Decodes an instruction from the instruction stream provided by the fetcher.
    pub fn decode<FETCH: Fetch<E = E>, E>(&mut self, is: &mut FETCH) -> Result<IrOp, E> {
        let mut context = DecoderContext::new(self.use32);
        let opcode = loop {
            let opcode = is.fetch_u8()?;
            match opcode {
                0x2e => {
                    // CS segment override prefix
                    context.segment_override = Some(CS);
                }
                0x36 => {
                    // SS segment override prefix
                    context.segment_override = Some(SS);
                }
                0x3e => {
                    // DS segment override prefix
                    context.segment_override = Some(DS);
                }
                0x26 => {
                    // ES segment override prefix
                    context.segment_override = Some(ES);
                }
                0x64 => {
                    // FS segment override prefix
                    context.segment_override = Some(FS);
                }
                0x65 => {
                    // GS segment override prefix
                    context.segment_override = Some(GS);
                }
                0x66 => {
                    // Operand-size override prefix
                    context.is_opsize32 = !self.use32;
                }
                0x67 => {
                    // Address-size override prefix
                    context.is_addr32 = !self.use32;
                }
                0xf0 => {
                    // Consume LOCK without validation or changing REP state.
                }
                0xf2 => {
                    // REPNE/REPNZ prefix
                    context.repeat_prefix = RepeatPrefix::F2;
                }
                0xf3 => {
                    // REP/REPE/REPZ prefix
                    context.repeat_prefix = RepeatPrefix::F3;
                }
                _ => {
                    break opcode;
                }
            }
        };

        match opcode {
            0x00 => {
                // ADD r/m8, reg8
                context.modrm_rb(is, IrOp::ADD_Rb_Rb, IrOp::ADD_Mb_Rb)
            }
            0x01 => {
                // ADD r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::ADD_Rw_Rw,
                    IrOp::ADD_Mw_Rw,
                    IrOp::ADD_Rd_Rd,
                    IrOp::ADD_Md_Rd,
                )
            }
            0x02 => {
                // ADD reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::ADD_Rb_Rb(r, m),
                    |m, r| IrOp::ADD_Rb_Mb(r, m),
                )
            }
            0x03 => {
                // ADD reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::ADD_Rw_Rw(r, m),
                    |m, r| IrOp::ADD_Rw_Mw(r, m),
                    |m, r| IrOp::ADD_Rd_Rd(r, m),
                    |m, r| IrOp::ADD_Rd_Md(r, m),
                )
            }
            0x04 => {
                // ADD AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::ADD_Rb_Ib(GprIndex8::AL, imm))
            }
            0x05 => {
                if context.is_opsize32 {
                    // ADD EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::ADD_Rd_Id(EAX, imm))
                } else {
                    // ADD AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::ADD_Rw_Iw(AX, imm))
                }
            }
            0x06 => {
                // PUSH ES
                Ok(if context.is_opsize32 {
                    IrOp::PUSH_SrO32(ES)
                } else {
                    IrOp::PUSH_SrO16(ES)
                })
            }
            0x07 => {
                // POP ES
                Ok(if context.is_opsize32 {
                    IrOp::POP_SrO32(ES)
                } else {
                    IrOp::POP_SrO16(ES)
                })
            }
            0x08 => {
                // OR r/m8, reg8
                context.modrm_rb(is, IrOp::OR_Rb_Rb, IrOp::OR_Mb_Rb)
            }
            0x09 => {
                // OR r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::OR_Rw_Rw,
                    IrOp::OR_Mw_Rw,
                    IrOp::OR_Rd_Rd,
                    IrOp::OR_Md_Rd,
                )
            }
            0x0a => {
                // OR reg8, r/m8
                context.modrm_rb(is, |m, r| IrOp::OR_Rb_Rb(r, m), |m, r| IrOp::OR_Rb_Mb(r, m))
            }
            0x0b => {
                // OR reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::OR_Rw_Rw(r, m),
                    |m, r| IrOp::OR_Rw_Mw(r, m),
                    |m, r| IrOp::OR_Rd_Rd(r, m),
                    |m, r| IrOp::OR_Rd_Md(r, m),
                )
            }
            0x0c => {
                // OR AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::OR_Rb_Ib(GprIndex8::AL, imm))
            }
            0x0d => {
                if context.is_opsize32 {
                    // OR EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::OR_Rd_Id(EAX, imm))
                } else {
                    // OR AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::OR_Rw_Iw(AX, imm))
                }
            }
            0x0e => {
                // PUSH CS
                Ok(if context.is_opsize32 {
                    IrOp::PUSH_SrO32(CS)
                } else {
                    IrOp::PUSH_SrO16(CS)
                })
            }

            0x10 => {
                // ADC r/m8, reg8
                context.modrm_rb(is, IrOp::ADC_Rb_Rb, IrOp::ADC_Mb_Rb)
            }
            0x11 => {
                // ADC r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::ADC_Rw_Rw,
                    IrOp::ADC_Mw_Rw,
                    IrOp::ADC_Rd_Rd,
                    IrOp::ADC_Md_Rd,
                )
            }
            0x12 => {
                // ADC reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::ADC_Rb_Rb(r, m),
                    |m, r| IrOp::ADC_Rb_Mb(r, m),
                )
            }
            0x13 => {
                // ADC reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::ADC_Rw_Rw(r, m),
                    |m, r| IrOp::ADC_Rw_Mw(r, m),
                    |m, r| IrOp::ADC_Rd_Rd(r, m),
                    |m, r| IrOp::ADC_Rd_Md(r, m),
                )
            }
            0x14 => {
                // ADC AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::ADC_Rb_Ib(GprIndex8::AL, imm))
            }
            0x15 => {
                if context.is_opsize32 {
                    // ADC EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::ADC_Rd_Id(EAX, imm))
                } else {
                    // ADC AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::ADC_Rw_Iw(AX, imm))
                }
            }
            0x16 => {
                // PUSH SS
                Ok(if context.is_opsize32 {
                    IrOp::PUSH_SrO32(SS)
                } else {
                    IrOp::PUSH_SrO16(SS)
                })
            }
            0x17 => {
                // POP SS
                Ok(if context.is_opsize32 {
                    IrOp::POP_SrO32(SS)
                } else {
                    IrOp::POP_SrO16(SS)
                })
            }
            0x18 => {
                // SBB r/m8, reg8
                context.modrm_rb(is, IrOp::SBB_Rb_Rb, IrOp::SBB_Mb_Rb)
            }
            0x19 => {
                // SBB r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::SBB_Rw_Rw,
                    IrOp::SBB_Mw_Rw,
                    IrOp::SBB_Rd_Rd,
                    IrOp::SBB_Md_Rd,
                )
            }
            0x1a => {
                // SBB reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::SBB_Rb_Rb(r, m),
                    |m, r| IrOp::SBB_Rb_Mb(r, m),
                )
            }
            0x1b => {
                // SBB reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::SBB_Rw_Rw(r, m),
                    |m, r| IrOp::SBB_Rw_Mw(r, m),
                    |m, r| IrOp::SBB_Rd_Rd(r, m),
                    |m, r| IrOp::SBB_Rd_Md(r, m),
                )
            }
            0x1c => {
                // SBB AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::SBB_Rb_Ib(GprIndex8::AL, imm))
            }
            0x1d => {
                if context.is_opsize32 {
                    // SBB EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::SBB_Rd_Id(EAX, imm))
                } else {
                    // SBB AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::SBB_Rw_Iw(AX, imm))
                }
            }
            0x1e => {
                // PUSH DS
                Ok(if context.is_opsize32 {
                    IrOp::PUSH_SrO32(DS)
                } else {
                    IrOp::PUSH_SrO16(DS)
                })
            }
            0x1f => {
                // POP DS
                Ok(if context.is_opsize32 {
                    IrOp::POP_SrO32(DS)
                } else {
                    IrOp::POP_SrO16(DS)
                })
            }

            0x20 => {
                // AND r/m8, reg8
                context.modrm_rb(is, IrOp::AND_Rb_Rb, IrOp::AND_Mb_Rb)
            }
            0x21 => {
                // AND r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::AND_Rw_Rw,
                    IrOp::AND_Mw_Rw,
                    IrOp::AND_Rd_Rd,
                    IrOp::AND_Md_Rd,
                )
            }
            0x22 => {
                // AND reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::AND_Rb_Rb(r, m),
                    |m, r| IrOp::AND_Rb_Mb(r, m),
                )
            }
            0x23 => {
                // AND reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::AND_Rw_Rw(r, m),
                    |m, r| IrOp::AND_Rw_Mw(r, m),
                    |m, r| IrOp::AND_Rd_Rd(r, m),
                    |m, r| IrOp::AND_Rd_Md(r, m),
                )
            }
            0x24 => {
                // AND AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::AND_Rb_Ib(GprIndex8::AL, imm))
            }
            0x25 => {
                if context.is_opsize32 {
                    // AND EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::AND_Rd_Id(EAX, imm))
                } else {
                    // AND AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::AND_Rw_Iw(AX, imm))
                }
            }
            // 0x26 es:
            0x27 => {
                // DAA
                Ok(IrOp::DAA)
            }
            0x28 => {
                // SUB r/m8, reg8
                context.modrm_rb(is, IrOp::SUB_Rb_Rb, IrOp::SUB_Mb_Rb)
            }
            0x29 => {
                // SUB r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::SUB_Rw_Rw,
                    IrOp::SUB_Mw_Rw,
                    IrOp::SUB_Rd_Rd,
                    IrOp::SUB_Md_Rd,
                )
            }
            0x2a => {
                // SUB reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::SUB_Rb_Rb(r, m),
                    |m, r| IrOp::SUB_Rb_Mb(r, m),
                )
            }
            0x2b => {
                // SUB reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::SUB_Rw_Rw(r, m),
                    |m, r| IrOp::SUB_Rw_Mw(r, m),
                    |m, r| IrOp::SUB_Rd_Rd(r, m),
                    |m, r| IrOp::SUB_Rd_Md(r, m),
                )
            }
            0x2c => {
                // SUB AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::SUB_Rb_Ib(GprIndex8::AL, imm))
            }
            0x2d => {
                if context.is_opsize32 {
                    // SUB EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::SUB_Rd_Id(EAX, imm))
                } else {
                    // SUB AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::SUB_Rw_Iw(AX, imm))
                }
            }
            // 0x2e cs:
            0x2f => {
                // DAS
                Ok(IrOp::DAS)
            }

            0x30 => {
                // XOR r/m8, reg8
                context.modrm_rb(is, IrOp::XOR_Rb_Rb, IrOp::XOR_Mb_Rb)
            }
            0x31 => {
                // XOR r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::XOR_Rw_Rw,
                    IrOp::XOR_Mw_Rw,
                    IrOp::XOR_Rd_Rd,
                    IrOp::XOR_Md_Rd,
                )
            }
            0x32 => {
                // XOR reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::XOR_Rb_Rb(r, m),
                    |m, r| IrOp::XOR_Rb_Mb(r, m),
                )
            }
            0x33 => {
                // XOR reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::XOR_Rw_Rw(r, m),
                    |m, r| IrOp::XOR_Rw_Mw(r, m),
                    |m, r| IrOp::XOR_Rd_Rd(r, m),
                    |m, r| IrOp::XOR_Rd_Md(r, m),
                )
            }
            0x34 => {
                // XOR AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::XOR_Rb_Ib(GprIndex8::AL, imm))
            }
            0x35 => {
                if context.is_opsize32 {
                    // XOR EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::XOR_Rd_Id(EAX, imm))
                } else {
                    // XOR AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::XOR_Rw_Iw(AX, imm))
                }
            }
            // 0x36 ss:
            0x37 => {
                // AAA
                Ok(IrOp::AAA)
            }

            0x38 => {
                // CMP r/m8, reg8
                context.modrm_rb(is, IrOp::CMP_Rb_Rb, IrOp::CMP_Mb_Rb)
            }
            0x39 => {
                // CMP r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::CMP_Rw_Rw,
                    IrOp::CMP_Mw_Rw,
                    IrOp::CMP_Rd_Rd,
                    IrOp::CMP_Md_Rd,
                )
            }
            0x3a => {
                // CMP reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::CMP_Rb_Rb(r, m),
                    |m, r| IrOp::CMP_Rb_Mb(r, m),
                )
            }
            0x3b => {
                // CMP reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::CMP_Rw_Rw(r, m),
                    |m, r| IrOp::CMP_Rw_Mw(r, m),
                    |m, r| IrOp::CMP_Rd_Rd(r, m),
                    |m, r| IrOp::CMP_Rd_Md(r, m),
                )
            }
            0x3c => {
                // CMP AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::CMP_Rb_Ib(GprIndex8::AL, imm))
            }
            0x3d => {
                if context.is_opsize32 {
                    // CMP EAX, imm32
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::CMP_Rd_Id(EAX, imm))
                } else {
                    // CMP AX, imm16/32
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::CMP_Rw_Iw(AX, imm))
                }
            }
            // 0x3e ds:
            0x3f => {
                // AAS
                Ok(IrOp::AAS)
            }

            0x40..=0x47 => {
                // INC reg16/32
                let reg_index = GprIndex32::from_u8(opcode);
                if context.is_opsize32 {
                    // INC reg32
                    Ok(IrOp::INC_Rd(reg_index))
                } else {
                    // INC reg16
                    Ok(IrOp::INC_Rw(reg_index.downgrade()))
                }
            }
            0x48..=0x4f => {
                // DEC reg16/32
                let reg_index = GprIndex32::from_u8(opcode);
                if context.is_opsize32 {
                    // DEC reg32
                    Ok(IrOp::DEC_Rd(reg_index))
                } else {
                    // DEC reg16
                    Ok(IrOp::DEC_Rw(reg_index.downgrade()))
                }
            }
            0x50..=0x57 => {
                // PUSH reg16/32
                let reg_index = GprIndex32::from_u8(opcode);
                if context.is_opsize32 {
                    // PUSH reg32
                    Ok(IrOp::PUSH_Rd(reg_index))
                } else {
                    // PUSH reg16
                    Ok(IrOp::PUSH_Rw(reg_index.downgrade()))
                }
            }
            0x58..=0x5f => {
                // POP reg16/32
                let reg_index = GprIndex32::from_u8(opcode);
                if context.is_opsize32 {
                    // POP reg32
                    Ok(IrOp::POP_Rd(reg_index))
                } else {
                    // POP reg16
                    Ok(IrOp::POP_Rw(reg_index.downgrade()))
                }
            }

            0x60 => {
                // PUSHA/PUSHAD
                if context.is_opsize32 {
                    Ok(IrOp::PUSHAD)
                } else {
                    Ok(IrOp::PUSHA)
                }
            }
            0x61 => {
                // POPA/POPAD
                if context.is_opsize32 {
                    Ok(IrOp::POPAD)
                } else {
                    Ok(IrOp::POPA)
                }
            }
            0x62 => {
                // BOUND r16/32, m16/32
                context.modrm_mv(
                    is,
                    |_| IrOp::UD,
                    |m, r| IrOp::BOUND_Rw_Mw(r, m),
                    |m, r| IrOp::BOUND_Rd_Md(r, m),
                )
            }
            0x63 => {
                // ARPL r16/32, r/m16/32
                context.modrm_rw(is, IrOp::ARPL_Rw_Rw, IrOp::ARPL_Mw_Rw)
            }
            // 0x64 fs:
            // 0x65 gs:
            // 0x66 operand-size override prefix
            // 0x67 address-size override prefix
            0x68 => {
                // PUSH imm16/32
                if context.is_opsize32 {
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::PUSH_Id(imm))
                } else {
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::PUSH_Iw(imm))
                }
            }
            0x69 => {
                // IMUL r16/32, r/m16/32, imm16/32
                context.modrm_rv_sub3(
                    is,
                    |is, s, c| {
                        let imm = is.fetch_u16()?;
                        Ok(IrOp::IMUL_Rw_Rw_Iw(GprIndex16::from_idx3(c), s, imm))
                    },
                    |is, s, c| {
                        let imm = is.fetch_u16()?;
                        Ok(IrOp::IMUL_Rw_Mw_Iw(GprIndex16::from_idx3(c), s, imm))
                    },
                    |is, s, c| {
                        let imm = is.fetch_u32()?;
                        Ok(IrOp::IMUL_Rd_Rd_Id(GprIndex32::from_idx3(c), s, imm))
                    },
                    |is, s, c| {
                        let imm = is.fetch_u32()?;
                        Ok(IrOp::IMUL_Rd_Md_Id(GprIndex32::from_idx3(c), s, imm))
                    },
                )
            }
            0x6a => {
                // PUSH imm8
                let imm = is.fetch_i8()? as i32;
                if context.is_opsize32 {
                    Ok(IrOp::PUSH_Id(imm as u32))
                } else {
                    Ok(IrOp::PUSH_Iw(imm as u16))
                }
            }
            0x6b => {
                // IMUL r16/32, r/m16/32, imm8
                context.modrm_rv_ib(
                    is,
                    |m, r, imm| IrOp::IMUL_Rw_Rw_Iw(r, m, imm as i8 as i16 as u16),
                    |m, r, imm| IrOp::IMUL_Rw_Mw_Iw(r, m, imm as i8 as i16 as u16),
                    |m, r, imm| IrOp::IMUL_Rd_Rd_Id(r, m, imm as i8 as i32 as u32),
                    |m, r, imm| IrOp::IMUL_Rd_Md_Id(r, m, imm as i8 as i32 as u32),
                )
            }
            0x6c => {
                // INS m8, DX
                match context.repeat_prefix {
                    RepeatPrefix::None => Ok(IrOp::INSB(StringOprEs {
                        is_addr32: context.is_addr32,
                    })),
                    RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_INSB(StringOprEs {
                        is_addr32: context.is_addr32,
                    })),
                }
            }
            0x6d => {
                // INS m16/32, DX
                if context.is_opsize32 {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::INSD(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_INSD(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                    }
                } else {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::INSW(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_INSW(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                    }
                }
            }
            0x6e => {
                // OUTS DX, m8
                match context.repeat_prefix {
                    RepeatPrefix::None => Ok(IrOp::OUTSB(StringOpr {
                        segment: context.segment_override.unwrap_or(DS),
                        is_addr32: context.is_addr32,
                    })),
                    RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_OUTSB(StringOpr {
                        segment: context.segment_override.unwrap_or(DS),
                        is_addr32: context.is_addr32,
                    })),
                }
            }
            0x6f => {
                // OUTS DX, m16/32
                if context.is_opsize32 {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::OUTSD(StringOpr {
                            segment: context.segment_override.unwrap_or(DS),
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_OUTSD(StringOpr {
                            segment: context.segment_override.unwrap_or(DS),
                            is_addr32: context.is_addr32,
                        })),
                    }
                } else {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::OUTSW(StringOpr {
                            segment: context.segment_override.unwrap_or(DS),
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_OUTSW(StringOpr {
                            segment: context.segment_override.unwrap_or(DS),
                            is_addr32: context.is_addr32,
                        })),
                    }
                }
            }

            0x70..=0x7f => {
                // Jcc rel8
                let rel = is.fetch_i8()? as i32;
                let cc = CC::from_u8(opcode);
                let target = is.current_eip().0.wrapping_add(rel as u32);
                Ok(if context.is_opsize32 {
                    IrOp::JCC_Jd(cc, Offset32(target))
                } else {
                    IrOp::JCC_Jw(cc, Offset16(target as u16))
                })
            }

            0x80 | 0x82 => {
                // Grp1 r/m8, imm8
                // NOTE: 0x82 is strictly undefined, but treated the same as 0x80 by most CPUs.
                context.modrm_rb_sub3(
                    is,
                    move |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u8()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Rb_Ib(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Rb_Ib(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Rb_Ib(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Rb_Ib(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Rb_Ib(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Rb_Ib(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Rb_Ib(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Rb_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u8()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Mb_Ib(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Mb_Ib(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Mb_Ib(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Mb_Ib(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Mb_Ib(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Mb_Ib(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Mb_Ib(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Mb_Ib(s, imm)),
                        }
                    },
                )
            }
            0x81 => {
                // Grp1 r/m16/32, imm16/32
                context.modrm_rv_sub3(
                    is,
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u16()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Rw_Iw(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Rw_Iw(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Rw_Iw(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Rw_Iw(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Rw_Iw(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Rw_Iw(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Rw_Iw(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Rw_Iw(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u16()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Mw_Iw(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Mw_Iw(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Mw_Iw(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Mw_Iw(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Mw_Iw(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Mw_Iw(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Mw_Iw(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Mw_Iw(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u32()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Rd_Id(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Rd_Id(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Rd_Id(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Rd_Id(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Rd_Id(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Rd_Id(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Rd_Id(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Rd_Id(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_u32()?;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Md_Id(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Md_Id(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Md_Id(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Md_Id(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Md_Id(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Md_Id(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Md_Id(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Md_Id(s, imm)),
                        }
                    },
                )
            }
            0x83 => {
                // Grp1 r/m16/32, imm8
                context.modrm_rv_sub3(
                    is,
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_i8()? as i16 as u16;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Rw_Iw(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Rw_Iw(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Rw_Iw(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Rw_Iw(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Rw_Iw(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Rw_Iw(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Rw_Iw(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Rw_Iw(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_i8()? as i16 as u16;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Mw_Iw(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Mw_Iw(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Mw_Iw(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Mw_Iw(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Mw_Iw(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Mw_Iw(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Mw_Iw(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Mw_Iw(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_i8()? as i32 as u32;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Rd_Id(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Rd_Id(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Rd_Id(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Rd_Id(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Rd_Id(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Rd_Id(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Rd_Id(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Rd_Id(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp1::from_idx3(c);
                        let imm = is.fetch_i8()? as i32 as u32;
                        match sub_op {
                            OpGrp1::ADD => Ok(IrOp::ADD_Md_Id(s, imm)),
                            OpGrp1::OR => Ok(IrOp::OR_Md_Id(s, imm)),
                            OpGrp1::ADC => Ok(IrOp::ADC_Md_Id(s, imm)),
                            OpGrp1::SBB => Ok(IrOp::SBB_Md_Id(s, imm)),
                            OpGrp1::AND => Ok(IrOp::AND_Md_Id(s, imm)),
                            OpGrp1::SUB => Ok(IrOp::SUB_Md_Id(s, imm)),
                            OpGrp1::XOR => Ok(IrOp::XOR_Md_Id(s, imm)),
                            OpGrp1::CMP => Ok(IrOp::CMP_Md_Id(s, imm)),
                        }
                    },
                )
            }
            0x84 => {
                // TEST r/m8, reg8
                context.modrm_rb(is, IrOp::TEST_Rb_Rb, IrOp::TEST_Mb_Rb)
            }
            0x85 => {
                // TEST r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::TEST_Rw_Rw,
                    IrOp::TEST_Mw_Rw,
                    IrOp::TEST_Rd_Rd,
                    IrOp::TEST_Md_Rd,
                )
            }
            0x86 => {
                // XCHG r/m8, reg8
                context.modrm_rb(is, IrOp::XCHG_Rb_Rb, IrOp::XCHG_Mb_Rb)
            }
            0x87 => {
                // XCHG r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::XCHG_Rw_Rw,
                    IrOp::XCHG_Mw_Rw,
                    IrOp::XCHG_Rd_Rd,
                    IrOp::XCHG_Md_Rd,
                )
            }
            0x88 => {
                // MOV r/m8, reg8
                context.modrm_rb(is, IrOp::MOV_Rb_Rb, IrOp::MOV_Mb_Rb)
            }
            0x89 => {
                // MOV r/m16/32, reg16/32
                context.modrm_rv(
                    is,
                    IrOp::MOV_Rw_Rw,
                    IrOp::MOV_Mw_Rw,
                    IrOp::MOV_Rd_Rd,
                    IrOp::MOV_Md_Rd,
                )
            }
            0x8a => {
                // MOV reg8, r/m8
                context.modrm_rb(
                    is,
                    |m, r| IrOp::MOV_Rb_Rb(r, m),
                    |m, r| IrOp::MOV_Rb_Mb(r, m),
                )
            }
            0x8b => {
                // MOV reg16/32, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| IrOp::MOV_Rw_Rw(r, m),
                    |m, r| IrOp::MOV_Rw_Mw(r, m),
                    |m, r| IrOp::MOV_Rd_Rd(r, m),
                    |m, r| IrOp::MOV_Rd_Md(r, m),
                )
            }
            0x8c => {
                // MOV r/m16/32, Sreg
                context.modrm_rv(
                    is,
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Rw_Sr(m, sr)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Mw_Sr(m, sr)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Rd_Sr(m, sr)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Md_Sr(m, sr)
                        } else {
                            IrOp::UD
                        }
                    },
                )
            }
            0x8d => {
                // LEA reg16/32, mem
                context.modrm_mv(
                    is,
                    |_| IrOp::UD,
                    |m, r| IrOp::LEA_Rw_Mw(r, m),
                    |m, r| IrOp::LEA_Rd_Md(r, m),
                )
            }
            0x8e => {
                // MOV Sreg, r/m16/32
                context.modrm_rv(
                    is,
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Sr_Rw(sr, m)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Sr_Mw(sr, m)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Sr_Rd(sr, m)
                        } else {
                            IrOp::UD
                        }
                    },
                    |m, r| {
                        if let Some(sr) = SrIndex::from_u8(r as u8) {
                            IrOp::MOV_Sr_Md(sr, m)
                        } else {
                            IrOp::UD
                        }
                    },
                )
            }
            0x8f => {
                // POP r/m16/32
                context.modrm_rv_sub3(
                    is,
                    |_, s, c| match c {
                        Index3::I000 => Ok(IrOp::POP_Rw(s)),
                        _ => Ok(IrOp::UD),
                    },
                    |_, s, c| match c {
                        Index3::I000 => Ok(IrOp::POP_Mw(s)),
                        _ => Ok(IrOp::UD),
                    },
                    |_, s, c| match c {
                        Index3::I000 => Ok(IrOp::POP_Rd(s)),
                        _ => Ok(IrOp::UD),
                    },
                    |_, s, c| match c {
                        Index3::I000 => Ok(IrOp::POP_Md(s)),
                        _ => Ok(IrOp::UD),
                    },
                )
            }

            0x90 => {
                // NOP/PAUSE
                if matches!(context.repeat_prefix, RepeatPrefix::F3) {
                    Ok(IrOp::PAUSE)
                } else {
                    Ok(IrOp::NOP)
                }
            }
            0x91..=0x97 => {
                // XCHG reg16/32, AX/EAX
                let reg_index = GprIndex32::from_u8(opcode);
                if context.is_opsize32 {
                    Ok(IrOp::XCHG_Rd_Rd(EAX, reg_index))
                } else {
                    Ok(IrOp::XCHG_Rw_Rw(AX, reg_index.downgrade()))
                }
            }
            0x98 => {
                // CBW/CWDE
                if context.is_opsize32 {
                    Ok(IrOp::MOVSX_Rd_Rw(EAX, AX))
                } else {
                    Ok(IrOp::MOVSX_Rw_Rb(AX, AL))
                }
            }
            0x99 => {
                // CWD/CDQ
                if context.is_opsize32 {
                    Ok(IrOp::CDQ)
                } else {
                    Ok(IrOp::CWD)
                }
            }
            0x9a => {
                // CALL ptr16:16/32
                Ok(if context.is_opsize32 {
                    let offset = Offset32(is.fetch_u32()?);
                    let segment = SegmentSelector(is.fetch_u16()?);
                    IrOp::CALLF_ApO32(segment, offset)
                } else {
                    let offset = Offset16(is.fetch_u16()?);
                    let segment = SegmentSelector(is.fetch_u16()?);
                    IrOp::CALLF_ApO16(segment, offset)
                })
            }
            0x9b => {
                // FWAIT/WAIT
                Ok(IrOp::WAIT)
            }
            0x9c => {
                // PUSHF/PUSHFD
                if context.is_opsize32 {
                    Ok(IrOp::PUSHFD)
                } else {
                    Ok(IrOp::PUSHF)
                }
            }
            0x9d => {
                // POPF/POPFD
                if context.is_opsize32 {
                    Ok(IrOp::POPFD)
                } else {
                    Ok(IrOp::POPF)
                }
            }
            0x9e => {
                // SAHF
                Ok(IrOp::SAHF)
            }
            0x9f => {
                // LAHF
                Ok(IrOp::LAHF)
            }

            0xa0 => {
                // MOV AL, moffs8
                Ok(IrOp::MOV_Rb_Mb(AL, context.fetch_moffs(is)?))
            }
            0xa1 => {
                // MOV AX/EAX, moffs8
                let memopr = context.fetch_moffs(is)?;
                if context.is_opsize32 {
                    Ok(IrOp::MOV_Rd_Md(EAX, memopr))
                } else {
                    Ok(IrOp::MOV_Rw_Mw(AX, memopr))
                }
            }
            0xa2 => {
                // MOV moffs8, AL
                Ok(IrOp::MOV_Mb_Rb(context.fetch_moffs(is)?, AL))
            }
            0xa3 => {
                // MOV moffs8, AX/EAX
                let memopr = context.fetch_moffs(is)?;
                if context.is_opsize32 {
                    Ok(IrOp::MOV_Md_Rd(memopr, EAX))
                } else {
                    Ok(IrOp::MOV_Mw_Rw(memopr, AX))
                }
            }
            0xa4 => {
                // MOVSB
                match context.repeat_prefix {
                    RepeatPrefix::None => Ok(IrOp::MOVSB(StringOpr {
                        segment: context.segment_override.unwrap_or(SrIndex::DS),
                        is_addr32: context.is_addr32,
                    })),
                    RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_MOVSB(StringOpr {
                        segment: context.segment_override.unwrap_or(SrIndex::DS),
                        is_addr32: context.is_addr32,
                    })),
                }
            }
            0xa5 => {
                // MOVSW/MOVSD
                if context.is_opsize32 {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::MOVSD(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_MOVSD(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                    }
                } else {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::MOVSW(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_MOVSW(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                    }
                }
            }
            0xa6 => {
                // CMPSB
                match context.repeat_prefix {
                    RepeatPrefix::None => Ok(IrOp::CMPSB(StringOpr {
                        segment: context.segment_override.unwrap_or(SrIndex::DS),
                        is_addr32: context.is_addr32,
                    })),
                    RepeatPrefix::F2 => Ok(IrOp::REPNZ_CMPSB(StringOpr {
                        segment: context.segment_override.unwrap_or(SrIndex::DS),
                        is_addr32: context.is_addr32,
                    })),
                    RepeatPrefix::F3 => Ok(IrOp::REPZ_CMPSB(StringOpr {
                        segment: context.segment_override.unwrap_or(SrIndex::DS),
                        is_addr32: context.is_addr32,
                    })),
                }
            }
            0xa7 => {
                // CMPSW/CMPSD
                if context.is_opsize32 {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::CMPSD(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 => Ok(IrOp::REPNZ_CMPSD(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F3 => Ok(IrOp::REPZ_CMPSD(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                    }
                } else {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::CMPSW(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 => Ok(IrOp::REPNZ_CMPSW(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F3 => Ok(IrOp::REPZ_CMPSW(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                    }
                }
            }
            0xa8 => {
                // TEST AL, imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::TEST_Rb_Ib(AL, imm))
            }
            0xa9 => {
                // TEST AX/EAX, imm16/32
                if context.is_opsize32 {
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::TEST_Rd_Id(EAX, imm))
                } else {
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::TEST_Rw_Iw(AX, imm))
                }
            }
            0xaa => {
                // STOSB
                match context.repeat_prefix {
                    RepeatPrefix::None => Ok(IrOp::STOSB(StringOprEs {
                        is_addr32: context.is_addr32,
                    })),
                    RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_STOSB(StringOprEs {
                        is_addr32: context.is_addr32,
                    })),
                }
            }
            0xab => {
                // STOSW/STOSD
                if context.is_opsize32 {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::STOSD(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_STOSD(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                    }
                } else {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::STOSW(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_STOSW(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                    }
                }
            }
            0xac => {
                // LODSB
                match context.repeat_prefix {
                    RepeatPrefix::None => Ok(IrOp::LODSB(StringOpr {
                        segment: context.segment_override.unwrap_or(SrIndex::DS),
                        is_addr32: context.is_addr32,
                    })),
                    RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_LODSB(StringOpr {
                        segment: context.segment_override.unwrap_or(SrIndex::DS),
                        is_addr32: context.is_addr32,
                    })),
                }
            }
            0xad => {
                // LODSW/LODSD
                if context.is_opsize32 {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::LODSD(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_LODSD(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                    }
                } else {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::LODSW(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 | RepeatPrefix::F3 => Ok(IrOp::REP_LODSW(StringOpr {
                            segment: context.segment_override.unwrap_or(SrIndex::DS),
                            is_addr32: context.is_addr32,
                        })),
                    }
                }
            }
            0xae => {
                // SCASB
                match context.repeat_prefix {
                    RepeatPrefix::None => Ok(IrOp::SCASB(StringOprEs {
                        is_addr32: context.is_addr32,
                    })),
                    RepeatPrefix::F2 => Ok(IrOp::REPNZ_SCASB(StringOprEs {
                        is_addr32: context.is_addr32,
                    })),
                    RepeatPrefix::F3 => Ok(IrOp::REPZ_SCASB(StringOprEs {
                        is_addr32: context.is_addr32,
                    })),
                }
            }
            0xaf => {
                // SCASW/SCASD
                if context.is_opsize32 {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::SCASD(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 => Ok(IrOp::REPNZ_SCASD(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F3 => Ok(IrOp::REPZ_SCASD(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                    }
                } else {
                    match context.repeat_prefix {
                        RepeatPrefix::None => Ok(IrOp::SCASW(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F2 => Ok(IrOp::REPNZ_SCASW(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                        RepeatPrefix::F3 => Ok(IrOp::REPZ_SCASW(StringOprEs {
                            is_addr32: context.is_addr32,
                        })),
                    }
                }
            }

            0xb0..=0xb7 => {
                // MOV reg8, imm8
                let reg_index = GprIndex8::from_u8(opcode);
                let imm = is.fetch_u8()?;
                Ok(IrOp::MOV_Rb_Ib(reg_index, imm))
            }
            0xb8..=0xbf => {
                // MOV reg16/32, imm16/32
                if context.is_opsize32 {
                    let reg_index = GprIndex32::from_u8(opcode);
                    let imm = is.fetch_u32()?;
                    Ok(IrOp::MOV_Rd_Id(reg_index, imm))
                } else {
                    let reg_index = GprIndex16::from_u8(opcode);
                    let imm = is.fetch_u16()?;
                    Ok(IrOp::MOV_Rw_Iw(reg_index, imm))
                }
            }

            0xc0 => {
                // Grp2 r/m8, imm8 (shift/rotate)
                context.modrm_rb_sub3(
                    is,
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rb_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_Rb_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_Rb_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_Rb_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rb_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_Rb_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_Rb_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Mb_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_Mb_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_Mb_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_Mb_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Mb_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_Mb_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_Mb_Ib(s, imm)),
                        }
                    },
                )
            }
            0xc1 => {
                // Grp2 r/m16/32, imm8 (shift/rotate)
                context.modrm_rv_sub3(
                    is,
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rw_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_Rw_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_Rw_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_Rw_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rw_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_Rw_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_Rw_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Mw_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_Mw_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_Mw_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_Mw_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Mw_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_Mw_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_Mw_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rd_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_Rd_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_Rd_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_Rd_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rd_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_Rd_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_Rd_Ib(s, imm)),
                        }
                    },
                    |is, s, c| {
                        let imm = is.fetch_u8()?;
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Md_Ib(s, imm)),
                            OpShift::ROR => Ok(IrOp::ROR_Md_Ib(s, imm)),
                            OpShift::RCL => Ok(IrOp::RCL_Md_Ib(s, imm)),
                            OpShift::RCR => Ok(IrOp::RCR_Md_Ib(s, imm)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Md_Ib(s, imm)),
                            OpShift::SHR => Ok(IrOp::SHR_Md_Ib(s, imm)),
                            OpShift::SAR => Ok(IrOp::SAR_Md_Ib(s, imm)),
                        }
                    },
                )
            }
            0xc2 => {
                // RET imm16
                let imm = is.fetch_u16()?;
                if context.is_opsize32 {
                    Ok(IrOp::RET_D32(imm))
                } else {
                    Ok(IrOp::RET_D16(imm))
                }
            }
            0xc3 => {
                // RET
                if context.is_opsize32 {
                    Ok(IrOp::RET_D32(0))
                } else {
                    Ok(IrOp::RET_D16(0))
                }
            }
            0xc4 => {
                // LES r16/32, m16:16/32
                context.modrm_mv(
                    is,
                    |_| IrOp::UD,
                    |m, r| IrOp::LES_Rw_Mw(r, m),
                    |m, r| IrOp::LES_Rd_Md(r, m),
                )
            }
            0xc5 => {
                // LDS r16/32, m16:16/32
                context.modrm_mv(
                    is,
                    |_| IrOp::UD,
                    |m, r| IrOp::LDS_Rw_Mw(r, m),
                    |m, r| IrOp::LDS_Rd_Md(r, m),
                )
            }
            0xc6 => {
                // MOV r/m8, imm8
                context.modrm_rb_sub3(
                    is,
                    |is, d, c| match c {
                        Index3::I000 => {
                            let imm = is.fetch_u8()?;
                            Ok(IrOp::MOV_Rb_Ib(d, imm))
                        }
                        _ => Ok(IrOp::UD),
                    },
                    |is, d, c| match c {
                        Index3::I000 => {
                            let imm = is.fetch_u8()?;
                            Ok(IrOp::MOV_Mb_Ib(d, imm))
                        }
                        _ => Ok(IrOp::UD),
                    },
                )
            }
            0xc7 => {
                // MOV r/m16/32, imm16/32
                context.modrm_rv_sub3(
                    is,
                    |is, d, c| match c {
                        Index3::I000 => {
                            let imm = is.fetch_u16()?;
                            Ok(IrOp::MOV_Rw_Iw(d, imm))
                        }
                        _ => Ok(IrOp::UD),
                    },
                    |is, d, _c| {
                        let imm = is.fetch_u16()?;
                        Ok(IrOp::MOV_Mw_Iw(d, imm))
                    },
                    |is, d, _c| {
                        let imm = is.fetch_u32()?;
                        Ok(IrOp::MOV_Rd_Id(d, imm))
                    },
                    |is, d, _c| {
                        let imm = is.fetch_u32()?;
                        Ok(IrOp::MOV_Md_Id(d, imm))
                    },
                )
            }
            0xc8 => {
                // ENTER imm16, imm8
                let iw = is.fetch_u16()?;
                let ib = is.fetch_u8()?;
                if context.is_opsize32 != self.use32 || context.is_addr32 != self.use32 {
                    // ENTER sizes are not yet carried by IR; overridden forms
                    // remain deferred for existing 32-bit-only consumers.
                    return Ok(IrOp::UD);
                }
                Ok(IrOp::ENTER_Iw_Ib(iw, ib))
            }
            0xc9 => {
                // LEAVE
                // TODO: support prefix 66
                Ok(IrOp::LEAVE)
            }
            0xca => {
                // RETF imm16
                let imm = is.fetch_u16()?;
                if context.is_opsize32 {
                    Ok(IrOp::RETF_D32(imm))
                } else {
                    Ok(IrOp::RETF_D16(imm))
                }
            }
            0xcb => {
                // RETF
                if context.is_opsize32 {
                    Ok(IrOp::RETF_D32(0))
                } else {
                    Ok(IrOp::RETF_D16(0))
                }
            }
            0xcc => {
                // INT3 (the one-byte form; CD 03 stays INT_Ib(3))
                Ok(IrOp::INT3)
            }
            0xcd => {
                // INT imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::INT_Ib(imm))
            }
            0xce => {
                // INTO
                Ok(IrOp::INTO)
            }
            0xcf => {
                // IRET
                if context.is_opsize32 {
                    Ok(IrOp::IRETD)
                } else {
                    Ok(IrOp::IRET)
                }
            }

            0xd0 => {
                // Grp2 r/m8, 1 (shift/rotate)
                context.modrm_rb_sub3(
                    is,
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rb_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_Rb_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_Rb_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_Rb_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rb_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_Rb_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_Rb_Ib(s, 1)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Mb_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_Mb_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_Mb_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_Mb_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Mb_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_Mb_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_Mb_Ib(s, 1)),
                        }
                    },
                )
            }
            0xd1 => {
                // Grp2 r/m16/32, 1 (shift/rotate)
                context.modrm_rv_sub3(
                    is,
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rw_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_Rw_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_Rw_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_Rw_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rw_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_Rw_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_Rw_Ib(s, 1)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Mw_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_Mw_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_Mw_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_Mw_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Mw_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_Mw_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_Mw_Ib(s, 1)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rd_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_Rd_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_Rd_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_Rd_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rd_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_Rd_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_Rd_Ib(s, 1)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Md_Ib(s, 1)),
                            OpShift::ROR => Ok(IrOp::ROR_Md_Ib(s, 1)),
                            OpShift::RCL => Ok(IrOp::RCL_Md_Ib(s, 1)),
                            OpShift::RCR => Ok(IrOp::RCR_Md_Ib(s, 1)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Md_Ib(s, 1)),
                            OpShift::SHR => Ok(IrOp::SHR_Md_Ib(s, 1)),
                            OpShift::SAR => Ok(IrOp::SAR_Md_Ib(s, 1)),
                        }
                    },
                )
            }
            0xd2 => {
                // Grp2 r/m8, CL (shift/rotate)
                context.modrm_rb_sub3(
                    is,
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rb_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_Rb_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_Rb_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_Rb_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rb_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_Rb_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_Rb_Cl(s)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Mb_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_Mb_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_Mb_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_Mb_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Mb_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_Mb_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_Mb_Cl(s)),
                        }
                    },
                )
            }
            0xd3 => {
                // Grp2 r/m16/32, CL (shift/rotate)
                context.modrm_rv_sub3(
                    is,
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rw_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_Rw_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_Rw_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_Rw_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rw_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_Rw_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_Rw_Cl(s)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Mw_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_Mw_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_Mw_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_Mw_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Mw_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_Mw_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_Mw_Cl(s)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Rd_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_Rd_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_Rd_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_Rd_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Rd_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_Rd_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_Rd_Cl(s)),
                        }
                    },
                    |_, s, c| {
                        let opx = OpShift::from_idx3(c);
                        match opx {
                            OpShift::ROL => Ok(IrOp::ROL_Md_Cl(s)),
                            OpShift::ROR => Ok(IrOp::ROR_Md_Cl(s)),
                            OpShift::RCL => Ok(IrOp::RCL_Md_Cl(s)),
                            OpShift::RCR => Ok(IrOp::RCR_Md_Cl(s)),
                            OpShift::_SAL | OpShift::SHL => Ok(IrOp::SHL_Md_Cl(s)),
                            OpShift::SHR => Ok(IrOp::SHR_Md_Cl(s)),
                            OpShift::SAR => Ok(IrOp::SAR_Md_Cl(s)),
                        }
                    },
                )
            }
            0xd4 => {
                // AAM imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::AAM_Ib(imm))
            }
            0xd5 => {
                // AAD imm8
                let imm = is.fetch_u8()?;
                Ok(IrOp::AAD_Ib(imm))
            }
            0xd6 => {
                // SALC
                Ok(IrOp::SALC)
            }
            0xd7 => {
                // XLAT
                let segment = context.segment_override.unwrap_or(DS);
                if context.is_addr32 {
                    Ok(IrOp::XLAT_Mb(MemOpr32::for_xlat(segment).into()))
                } else {
                    Ok(IrOp::XLAT_Mb(MemOpr16::for_xlat(segment).into()))
                }
            }

            0xd8..=0xdf => {
                // Retain ModR/M before normalizing the memory operand. Engines need
                // it for FOP, independently of operand/address size and prefixes.
                let (modrm, memory) = if context.is_addr32 {
                    let parsed = ModRm32::fetch(is, context.segment_override)?;
                    (
                        parsed.raw_modrm(),
                        match parsed.reg_or_mem() {
                            RegOrMem::Reg(_) => None,
                            RegOrMem::Mem(mem) => Some(MemOpr::A32(mem)),
                        },
                    )
                } else {
                    let parsed = ModRm16::fetch(is, context.segment_override)?;
                    (
                        parsed.raw_modrm(),
                        match parsed.reg_or_mem() {
                            RegOrMem::Reg(_) => None,
                            RegOrMem::Mem(mem) => Some(MemOpr::A16(mem)),
                        },
                    )
                };
                let opcode = EscOpcode {
                    opcode,
                    modrm: modrm.raw(),
                    is_opsize32: context.is_opsize32,
                };
                let reg = modrm.reg_index() as u8;
                Ok(match memory {
                    Some(mem) => IrOp::ESC_Mb(opcode, reg, mem),
                    None => IrOp::ESC_Rb(opcode, reg, GprIndex8::from_idx3(modrm.rm_index())),
                })
            }

            0xe0..=0xe3 => {
                let rel = is.fetch_i8()? as i32;
                let target = is.current_eip().0.wrapping_add(rel as u32);
                let addr32 = context.is_addr32;
                Ok(if context.is_opsize32 {
                    let target = Offset32(target);
                    match opcode {
                        0xe0 => IrOp::LOOPNZ_Jd(addr32, target),
                        0xe1 => IrOp::LOOPZ_Jd(addr32, target),
                        0xe2 => IrOp::LOOP_Jd(addr32, target),
                        _ if addr32 => IrOp::JECXZ_Jd(target),
                        _ => IrOp::JCXZ_Jd(target),
                    }
                } else {
                    let target = Offset16(target as u16);
                    match opcode {
                        0xe0 => IrOp::LOOPNZ_Jw(addr32, target),
                        0xe1 => IrOp::LOOPZ_Jw(addr32, target),
                        0xe2 => IrOp::LOOP_Jw(addr32, target),
                        _ if addr32 => IrOp::JECXZ_Jw(target),
                        _ => IrOp::JCXZ_Jw(target),
                    }
                })
            }

            0xe4 => {
                // IN AL, imm8
                let port = is.fetch_u8()?;
                Ok(IrOp::IN_Al_Ib(port))
            }
            0xe5 => {
                // IN AX, imm16/32
                let port = is.fetch_u8()?;
                if context.is_opsize32 {
                    Ok(IrOp::IN_Ad_Ib(port))
                } else {
                    Ok(IrOp::IN_Aw_Ib(port))
                }
            }
            0xe6 => {
                // OUT imm8, AL
                let port = is.fetch_u8()?;
                Ok(IrOp::OUT_Ib_Al(port))
            }
            0xe7 => {
                // OUT imm8, AX/EAX
                let port = is.fetch_u8()?;
                if context.is_opsize32 {
                    Ok(IrOp::OUT_Ib_Ad(port))
                } else {
                    Ok(IrOp::OUT_Ib_Aw(port))
                }
            }
            0xe8 => {
                // CALL rel16/32
                Ok(if context.is_opsize32 {
                    let rel = is.fetch_i32()?;
                    let target = Offset32(is.current_eip().0.wrapping_add(rel as u32));
                    IrOp::CALL_Jd(target)
                } else {
                    let rel = is.fetch_i16()?;
                    let target = Offset16(is.current_eip().0.wrapping_add(rel as u32) as u16);
                    IrOp::CALL_Jw(target)
                })
            }
            0xe9 => {
                // JMP rel16/32
                Ok(if context.is_opsize32 {
                    let rel = is.fetch_i32()?;
                    let target = Offset32(is.current_eip().0.wrapping_add(rel as u32));
                    IrOp::JMP_Jd(target)
                } else {
                    let rel = is.fetch_i16()?;
                    let target = Offset16(is.current_eip().0.wrapping_add(rel as u32) as u16);
                    IrOp::JMP_Jw(target)
                })
            }
            0xea => {
                // JMP ptr16:16/32
                Ok(if context.is_opsize32 {
                    let offset = Offset32(is.fetch_u32()?);
                    let segment = SegmentSelector(is.fetch_u16()?);
                    IrOp::JMPF_ApO32(segment, offset)
                } else {
                    let offset = Offset16(is.fetch_u16()?);
                    let segment = SegmentSelector(is.fetch_u16()?);
                    IrOp::JMPF_ApO16(segment, offset)
                })
            }
            0xeb => {
                // JMP rel8
                let rel = is.fetch_i8()? as i32;
                let target = is.current_eip().0.wrapping_add(rel as u32);
                Ok(if context.is_opsize32 {
                    IrOp::JMP_Jd(Offset32(target))
                } else {
                    IrOp::JMP_Jw(Offset16(target as u16))
                })
            }
            0xec => {
                // IN AL, DX
                Ok(IrOp::IN_Al_Dx)
            }
            0xed => {
                // IN AX/EAX, DX
                if context.is_opsize32 {
                    Ok(IrOp::IN_Ad_Dx)
                } else {
                    Ok(IrOp::IN_Aw_Dx)
                }
            }
            0xee => {
                // OUT DX, AL
                Ok(IrOp::OUT_Dx_Al)
            }
            0xef => {
                // OUT DX, AX/EAX
                if context.is_opsize32 {
                    Ok(IrOp::OUT_Dx_Ad)
                } else {
                    Ok(IrOp::OUT_Dx_Aw)
                }
            }

            // 0xf0: LOCK prefix
            0xf1 => {
                // INT1 (ICEBP)
                Ok(IrOp::ICEBP)
            }
            // 0xf2: REPNE/REPNZ prefix
            // 0xf3: REP/REPE/REPZ prefix
            0xf4 => {
                // HLT
                Ok(IrOp::HLT)
            }
            0xf5 => {
                // CMC
                Ok(IrOp::CMC)
            }
            0xf6 => {
                // grp3 r/m8
                context.modrm_rb_sub3(
                    is,
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);

                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u8()?;
                                Ok(IrOp::TEST_Rb_Ib(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_Rb(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_Rb(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_Rb(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_Rb(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_Rb(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_Rb(s)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);

                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u8()?;
                                Ok(IrOp::TEST_Mb_Ib(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_Mb(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_Mb(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_Mb(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_Mb(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_Mb(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_Mb(s)),
                        }
                    },
                )
            }
            0xf7 => {
                // grp3 r/m16/32
                context.modrm_rv_sub3(
                    is,
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);

                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u16()?;
                                Ok(IrOp::TEST_Rw_Iw(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_Rw(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_Rw(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_Rw(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_Rw(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_Rw(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_Rw(s)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);

                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u16()?;
                                Ok(IrOp::TEST_Mw_Iw(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_Mw(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_Mw(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_Mw(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_Mw(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_Mw(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_Mw(s)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);

                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u32()?;
                                Ok(IrOp::TEST_Rd_Id(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_Rd(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_Rd(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_Rd(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_Rd(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_Rd(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_Rd(s)),
                        }
                    },
                    |is, s, c| {
                        let sub_op = OpGrp3::from_idx3(c);

                        match sub_op {
                            OpGrp3::TEST | OpGrp3::_TEST1 => {
                                let imm = is.fetch_u32()?;
                                Ok(IrOp::TEST_Md_Id(s, imm))
                            }
                            OpGrp3::NOT => Ok(IrOp::NOT_Md(s)),
                            OpGrp3::NEG => Ok(IrOp::NEG_Md(s)),
                            OpGrp3::MUL => Ok(IrOp::MUL_Md(s)),
                            OpGrp3::IMUL => Ok(IrOp::IMUL_Md(s)),
                            OpGrp3::DIV => Ok(IrOp::DIV_Md(s)),
                            OpGrp3::IDIV => Ok(IrOp::IDIV_Md(s)),
                        }
                    },
                )
            }
            0xf8 => {
                // CLC
                Ok(IrOp::CLC)
            }
            0xf9 => {
                // STC
                Ok(IrOp::STC)
            }
            0xfa => {
                // CLI
                Ok(IrOp::CLI)
            }
            0xfb => {
                // STI
                Ok(IrOp::STI)
            }
            0xfc => {
                // CLD
                Ok(IrOp::CLD)
            }
            0xfd => {
                // STD
                Ok(IrOp::STD)
            }
            0xfe => {
                // Grp4 r/m8
                context.modrm_rb_sub3(
                    is,
                    |_, s, c| {
                        let sub_op = OpGrp4::from_idx3(c);
                        match sub_op {
                            OpGrp4::INC => Ok(IrOp::INC_Rb(s)),
                            OpGrp4::DEC => Ok(IrOp::DEC_Rb(s)),
                            _ => Ok(IrOp::UD),
                        }
                    },
                    |_, s, c| {
                        let sub_op = OpGrp4::from_idx3(c);
                        match sub_op {
                            OpGrp4::INC => Ok(IrOp::INC_Mb(s)),
                            OpGrp4::DEC => Ok(IrOp::DEC_Mb(s)),
                            _ => Ok(IrOp::UD),
                        }
                    },
                )
            }
            0xff => {
                // Grp5 r/m16/32
                context.modrm_rv_sub3(
                    is,
                    |_, s, c| {
                        let sub_op = OpGrp5::from_idx3(c);
                        match sub_op {
                            OpGrp5::INC => Ok(IrOp::INC_Rw(s)),
                            OpGrp5::DEC => Ok(IrOp::DEC_Rw(s)),
                            OpGrp5::CALL => Ok(IrOp::CALL_Rw(s)),
                            OpGrp5::JMP => Ok(IrOp::JMP_Rw(s)),
                            OpGrp5::PUSH => Ok(IrOp::PUSH_Rw(s)),
                            _ => Ok(IrOp::UD),
                        }
                    },
                    |_, s, c| {
                        let sub_op = OpGrp5::from_idx3(c);
                        match sub_op {
                            OpGrp5::INC => Ok(IrOp::INC_Mw(s)),
                            OpGrp5::DEC => Ok(IrOp::DEC_Mw(s)),
                            OpGrp5::CALL => Ok(IrOp::CALL_Mw(s)),
                            OpGrp5::CALLF => Ok(IrOp::CALLF_MpO16(s)),
                            OpGrp5::JMP => Ok(IrOp::JMP_Mw(s)),
                            OpGrp5::JMPF => Ok(IrOp::JMPF_MpO16(s)),
                            OpGrp5::PUSH => Ok(IrOp::PUSH_Mw(s)),
                            OpGrp5::_UD7 => Ok(IrOp::UD),
                        }
                    },
                    |_, s, c| {
                        let sub_op = OpGrp5::from_idx3(c);
                        match sub_op {
                            OpGrp5::INC => Ok(IrOp::INC_Rd(s)),
                            OpGrp5::DEC => Ok(IrOp::DEC_Rd(s)),
                            OpGrp5::CALL => Ok(IrOp::CALL_Rd(s)),
                            OpGrp5::JMP => Ok(IrOp::JMP_Rd(s)),
                            OpGrp5::PUSH => Ok(IrOp::PUSH_Rd(s)),
                            _ => Ok(IrOp::UD),
                        }
                    },
                    |_, s, c| {
                        let sub_op = OpGrp5::from_idx3(c);
                        match sub_op {
                            OpGrp5::INC => Ok(IrOp::INC_Md(s)),
                            OpGrp5::DEC => Ok(IrOp::DEC_Md(s)),
                            OpGrp5::CALL => Ok(IrOp::CALL_Md(s)),
                            OpGrp5::CALLF => Ok(IrOp::CALLF_MpO32(s)),
                            OpGrp5::JMP => Ok(IrOp::JMP_Md(s)),
                            OpGrp5::JMPF => Ok(IrOp::JMPF_MpO32(s)),
                            OpGrp5::PUSH => Ok(IrOp::PUSH_Md(s)),
                            OpGrp5::_UD7 => Ok(IrOp::UD),
                        }
                    },
                )
            }

            0x0f => {
                // Two-byte opcode escape
                let second_opcode = is.fetch_u8()?;
                match second_opcode {
                    0x00 => {
                        // grp6
                        let o32 = context.is_opsize32;
                        context.modrm_rw_sub3(
                            is,
                            |_is, s, c| {
                                let sub_op = OpGrp6::from_idx3(c);
                                match sub_op {
                                    OpGrp6::SLDT if o32 => Ok(IrOp::SLDT_Rd(s.upgrade())),
                                    OpGrp6::STR if o32 => Ok(IrOp::STR_Rd(s.upgrade())),
                                    OpGrp6::SLDT => Ok(IrOp::SLDT_Rw(s)),
                                    OpGrp6::STR => Ok(IrOp::STR_Rw(s)),
                                    OpGrp6::LLDT => Ok(IrOp::LLDT_Rw(s)),
                                    OpGrp6::LTR => Ok(IrOp::LTR_Rw(s)),
                                    OpGrp6::VERR => Ok(IrOp::VERR_Rw(s)),
                                    OpGrp6::VERW => Ok(IrOp::VERW_Rw(s)),
                                    OpGrp6::_UD => Ok(IrOp::UD),
                                }
                            },
                            |_is, s, c| {
                                let sub_op = OpGrp6::from_idx3(c);
                                match sub_op {
                                    OpGrp6::SLDT if o32 => Ok(IrOp::SLDT_Md(s)),
                                    OpGrp6::STR if o32 => Ok(IrOp::STR_Md(s)),
                                    OpGrp6::SLDT => Ok(IrOp::SLDT_Mw(s)),
                                    OpGrp6::STR => Ok(IrOp::STR_Mw(s)),
                                    OpGrp6::LLDT => Ok(IrOp::LLDT_Mw(s)),
                                    OpGrp6::LTR => Ok(IrOp::LTR_Mw(s)),
                                    OpGrp6::VERR => Ok(IrOp::VERR_Mw(s)),
                                    OpGrp6::VERW => Ok(IrOp::VERW_Mw(s)),
                                    OpGrp6::_UD => Ok(IrOp::UD),
                                }
                            },
                        )
                    }
                    0x01 => {
                        // grp7
                        let o32 = context.is_opsize32;
                        context.modrm_mw_sub3(
                            is,
                            |_is, modrm| {
                                let sub_op = OpGrp7::from_idx3(modrm.reg_index());
                                let s = GprIndex16::from_idx3(modrm.rm_index());
                                match sub_op {
                                    OpGrp7::SMSW => Ok(IrOp::SMSW_Rw(s)),
                                    OpGrp7::LMSW => Ok(IrOp::LMSW_Rw(s)),
                                    _ => match modrm.raw() {
                                        // 0xc1 => Ok(IrOp::VMCALL),
                                        // 0xc2 => Ok(IrOp::VMLAUNCH),
                                        // 0xc3 => Ok(IrOp::VMRESUME),
                                        // 0xc4 => Ok(IrOp::VMXOFF),
                                        // 0xc8 => Ok(IrOp::MONITOR),
                                        // 0xc9 => Ok(IrOp::MWAIT),
                                        0xd0 => Ok(IrOp::XGETBV),
                                        0xd1 => Ok(IrOp::XSETBV),
                                        0xf8 => Ok(IrOp::SWAPGS),
                                        0xf9 => Ok(IrOp::RDTSCP),
                                        _ => Ok(IrOp::UD),
                                    },
                                }
                            },
                            |_is, s, c| {
                                let sub_op = OpGrp7::from_idx3(c);
                                match sub_op {
                                    OpGrp7::SGDT => Ok(IrOp::SGDT_Mp(s)),
                                    OpGrp7::SIDT => Ok(IrOp::SIDT_Mp(s)),
                                    OpGrp7::LGDT => Ok(if o32 {
                                        IrOp::LGDT_MpO32(s)
                                    } else {
                                        IrOp::LGDT_MpO16(s)
                                    }),
                                    OpGrp7::LIDT => Ok(if o32 {
                                        IrOp::LIDT_MpO32(s)
                                    } else {
                                        IrOp::LIDT_MpO16(s)
                                    }),
                                    OpGrp7::SMSW => Ok(IrOp::SMSW_Mw(s)),
                                    OpGrp7::LMSW => Ok(IrOp::LMSW_Mw(s)),
                                    OpGrp7::INVLPG => Ok(IrOp::INVLPG_Mw(s)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                        )
                    }

                    0x02 => {
                        // LAR r16/32, r/m16
                        context.modrm_rv(
                            is,
                            |s, d| IrOp::LAR_Rw_Rw(d, s),
                            |s, d| IrOp::LAR_Rw_Mw(d, s),
                            |s, d| IrOp::LAR_Rd_Rw(d, s.downgrade()),
                            |s, d| IrOp::LAR_Rd_Mw(d, s),
                        )
                    }
                    0x03 => {
                        // LSL r16/32, r/m16
                        context.modrm_rv(
                            is,
                            |s, d| IrOp::LSL_Rw_Rw(d, s),
                            |s, d| IrOp::LSL_Rw_Mw(d, s),
                            |s, d| IrOp::LSL_Rd_Rw(d, s.downgrade()),
                            |s, d| IrOp::LSL_Rd_Mw(d, s),
                        )
                    }
                    0x05 => {
                        // SYSCALL
                        Ok(IrOp::SYSCALL)
                    }
                    0x06 => {
                        // CLTS
                        Ok(IrOp::CLTS)
                    }
                    0x07 => {
                        // SYSRET
                        Ok(IrOp::SYSRET)
                    }
                    0x08 => {
                        // INVD
                        Ok(IrOp::INVD)
                    }
                    0x09 => {
                        // WBINVD
                        Ok(IrOp::WBINVD)
                    }

                    0x0b => {
                        // UD2
                        Ok(IrOp::UD)
                    }

                    0x0d => {
                        // NOP
                        context.modrm_mv(is, |_| IrOp::NOP, |_, _| IrOp::NOP, |_, _| IrOp::NOP)
                    }

                    0x18..=0x1f => {
                        // HINT_NOP Ev /?
                        context.modrm_rw_sub3(is, |_, _, _| Ok(IrOp::NOP), |_, _, _| Ok(IrOp::NOP))
                    }

                    0x20 => {
                        // MOV r/m16/32, CR0..CR7
                        let modrm = ModRM::new(is.fetch_u8()?);
                        if let Some(cr) = CrIndex::from_idx3(modrm.reg_index()) {
                            let reg = GprIndex32::from_idx3(modrm.rm_index());
                            Ok(IrOp::MOV_Rd_Cr(reg, cr))
                        } else {
                            Ok(IrOp::UD)
                        }
                    }
                    0x22 => {
                        // MOV CR0..CR7, r/m16/32
                        let modrm = ModRM::new(is.fetch_u8()?);
                        if let Some(cr) = CrIndex::from_idx3(modrm.reg_index()) {
                            let reg = GprIndex32::from_idx3(modrm.rm_index());
                            Ok(IrOp::MOV_Cr_Rd(cr, reg))
                        } else {
                            Ok(IrOp::UD)
                        }
                    }
                    0x21 | 0x23 => {
                        // Like MOV CR, MOV DR ignores ModR/M.mod and never fetches
                        // a SIB/displacement. Operand/address-size overrides do not
                        // change the 32-bit register transfer in non-64-bit modes.
                        let modrm = ModRM::new(is.fetch_u8()?);
                        let reg = GprIndex32::from_idx3(modrm.rm_index());
                        let debug = DrIndex::from_idx3(modrm.reg_index());
                        Ok(if second_opcode == 0x21 {
                            IrOp::MOV_Rd_Dr(reg, debug)
                        } else {
                            IrOp::MOV_Dr_Rd(debug, reg)
                        })
                    }

                    0x30 => {
                        // WRMSR
                        Ok(IrOp::WRMSR)
                    }
                    0x31 => {
                        // RDTSC
                        Ok(IrOp::RDTSC)
                    }
                    0x32 => {
                        // RDMSR
                        Ok(IrOp::RDMSR)
                    }

                    0x34 => {
                        // SYSENTER
                        Ok(IrOp::SYSENTER)
                    }
                    0x35 => {
                        // SYSEXIT
                        Ok(IrOp::SYSEXIT)
                    }

                    0x38 => {
                        // TODO: 3-byte opcode escape (0x0f 0x38)
                        Ok(IrOp::UD)
                    }

                    0x3a => {
                        // TODO: 3-byte opcode escape (0x0f 0x3a)
                        Ok(IrOp::UD)
                    }

                    0x40..=0x4f => {
                        // CMOVcc r16/32, r/m16/32
                        let cc = CC::from_u8(second_opcode);
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::CMOV_Rw_Rw(cc, r, m),
                            |m, r| IrOp::CMOV_Rw_Mw(cc, r, m),
                            |m, r| IrOp::CMOV_Rd_Rd(cc, r, m),
                            |m, r| IrOp::CMOV_Rd_Md(cc, r, m),
                        )
                    }

                    0x80..=0x8f => {
                        // Jcc rel16/32
                        let cc = CC::from_u8(second_opcode);
                        Ok(if context.is_opsize32 {
                            let rel = is.fetch_i32()?;
                            let target = Offset32(is.current_eip().0.wrapping_add(rel as u32));
                            IrOp::JCC_Jd(cc, target)
                        } else {
                            let rel = is.fetch_i16()?;
                            let target =
                                Offset16(is.current_eip().0.wrapping_add(rel as u32) as u16);
                            IrOp::JCC_Jw(cc, target)
                        })
                    }

                    0x90..=0x9f => {
                        // SETcc r/m8
                        let cc = CC::from_u8(second_opcode);
                        context.modrm_rb_sub3(
                            is,
                            |_, s, c| match c {
                                Index3::I000 => Ok(IrOp::SETCC_Rb(cc, s)),
                                _ => Ok(IrOp::UD),
                            },
                            |_, s, c| match c {
                                Index3::I000 => Ok(IrOp::SETCC_Mb(cc, s)),
                                _ => Ok(IrOp::UD),
                            },
                        )
                    }

                    0xa0 => {
                        // PUSH FS
                        Ok(if context.is_opsize32 {
                            IrOp::PUSH_SrO32(FS)
                        } else {
                            IrOp::PUSH_SrO16(FS)
                        })
                    }
                    0xa1 => {
                        // POP FS
                        Ok(if context.is_opsize32 {
                            IrOp::POP_SrO32(FS)
                        } else {
                            IrOp::POP_SrO16(FS)
                        })
                    }
                    0xa2 => {
                        // CPUID
                        Ok(IrOp::CPUID)
                    }
                    0xa3 => {
                        // BT r/m16/32, r16/32
                        context.modrm_rv(
                            is,
                            IrOp::BT_Rw_Rw,
                            IrOp::BT_Mw_Rw,
                            IrOp::BT_Rd_Rd,
                            IrOp::BT_Md_Rd,
                        )
                    }
                    0xa4 => {
                        // SHLD r/m16/32, r16/32, imm8
                        context.modrm_rv_ib(
                            is,
                            IrOp::SHLD_Rw_Rw_Ib,
                            IrOp::SHLD_Mw_Rw_Ib,
                            IrOp::SHLD_Rd_Rd_Ib,
                            IrOp::SHLD_Md_Rd_Ib,
                        )
                    }
                    0xa5 => {
                        // SHLD r/m16/32, r16/32, cl
                        context.modrm_rv(
                            is,
                            IrOp::SHLD_Rw_Rw_Cl,
                            IrOp::SHLD_Mw_Rw_Cl,
                            IrOp::SHLD_Rd_Rd_Cl,
                            IrOp::SHLD_Md_Rd_Cl,
                        )
                    }
                    0xa8 => {
                        // PUSH GS
                        Ok(if context.is_opsize32 {
                            IrOp::PUSH_SrO32(GS)
                        } else {
                            IrOp::PUSH_SrO16(GS)
                        })
                    }
                    0xa9 => {
                        // POP GS
                        Ok(if context.is_opsize32 {
                            IrOp::POP_SrO32(GS)
                        } else {
                            IrOp::POP_SrO16(GS)
                        })
                    }
                    0xaa => {
                        // RSM
                        Ok(IrOp::RSM)
                    }
                    0xab => {
                        // BTS r/m16/32, r16/32
                        context.modrm_rv(
                            is,
                            IrOp::BTS_Rw_Rw,
                            IrOp::BTS_Mw_Rw,
                            IrOp::BTS_Rd_Rd,
                            IrOp::BTS_Md_Rd,
                        )
                    }
                    0xac => {
                        // SHRD r/m16/32, r16/32, imm8
                        context.modrm_rv_ib(
                            is,
                            IrOp::SHRD_Rw_Rw_Ib,
                            IrOp::SHRD_Mw_Rw_Ib,
                            IrOp::SHRD_Rd_Rd_Ib,
                            IrOp::SHRD_Md_Rd_Ib,
                        )
                    }
                    0xad => {
                        // SHRD r/m16/32, r16/32, cl
                        context.modrm_rv(
                            is,
                            IrOp::SHRD_Rw_Rw_Cl,
                            IrOp::SHRD_Mw_Rw_Cl,
                            IrOp::SHRD_Rd_Rd_Cl,
                            IrOp::SHRD_Md_Rd_Cl,
                        )
                    }
                    // 0xae grp15
                    0xaf => {
                        // IMUL r16/32, r/m16/32
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::IMUL_Rw_Rw(r, m),
                            |m, r| IrOp::IMUL_Rw_Mw(r, m),
                            |m, r| IrOp::IMUL_Rd_Rd(r, m),
                            |m, r| IrOp::IMUL_Rd_Md(r, m),
                        )
                    }

                    0xb0 => {
                        // CMPXCHG r/m8, reg8
                        context.modrm_rb(is, IrOp::CMPXCHG_Rb_Rb, IrOp::CMPXCHG_Mb_Rb)
                    }
                    0xb1 => {
                        // CMPXCHG r/m16/32, reg16/32
                        context.modrm_rv(
                            is,
                            IrOp::CMPXCHG_Rw_Rw,
                            IrOp::CMPXCHG_Mw_Rw,
                            IrOp::CMPXCHG_Rd_Rd,
                            IrOp::CMPXCHG_Md_Rd,
                        )
                    }
                    0xb2 => {
                        // LSS r16/32, m16:16/32
                        context.modrm_mv(
                            is,
                            |_| IrOp::UD,
                            |m, r| IrOp::LSS_Rw_Mw(r, m),
                            |m, r| IrOp::LSS_Rd_Md(r, m),
                        )
                    }
                    0xb3 => {
                        // BTR r/m16/32, r16/32
                        context.modrm_rv(
                            is,
                            IrOp::BTR_Rw_Rw,
                            IrOp::BTR_Mw_Rw,
                            IrOp::BTR_Rd_Rd,
                            IrOp::BTR_Md_Rd,
                        )
                    }
                    0xb4 => {
                        // LFS r16/32, m16:16/32
                        context.modrm_mv(
                            is,
                            |_| IrOp::UD,
                            |m, r| IrOp::LFS_Rw_Mw(r, m),
                            |m, r| IrOp::LFS_Rd_Md(r, m),
                        )
                    }
                    0xb5 => {
                        // LGS r16/32, m16:16/32
                        context.modrm_mv(
                            is,
                            |_| IrOp::UD,
                            |m, r| IrOp::LGS_Rw_Mw(r, m),
                            |m, r| IrOp::LGS_Rd_Md(r, m),
                        )
                    }
                    0xb6 => {
                        // MOVZX r16/32, r/m8
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::MOVZX_Rw_Rb(r, GprIndex8::from_u8(m as u8)),
                            |m, r| IrOp::MOVZX_Rw_Mb(r, m),
                            |m, r| IrOp::MOVZX_Rd_Rb(r, GprIndex8::from_u8(m as u8)),
                            |m, r| IrOp::MOVZX_Rd_Mb(r, m),
                        )
                    }
                    0xb7 => {
                        // MOVZX r16/32, r/m16
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::MOV_Rw_Rw(r, m),
                            |m, r| IrOp::MOV_Rw_Mw(r, m),
                            |m, r| IrOp::MOVZX_Rd_Rw(r, m.downgrade()),
                            |m, r| IrOp::MOVZX_Rd_Mw(r, m),
                        )
                    }
                    0xb8 => {
                        if matches!(context.repeat_prefix, RepeatPrefix::F3) {
                            // POPCNT r16/32, r/m16/32
                            context.modrm_rv(
                                is,
                                |m, r| IrOp::POPCNT_Rw_Rw(r, m),
                                |m, r| IrOp::POPCNT_Rw_Mw(r, m),
                                |m, r| IrOp::POPCNT_Rd_Rd(r, m),
                                |m, r| IrOp::POPCNT_Rd_Md(r, m),
                            )
                        } else {
                            // JMPE?
                            Ok(IrOp::UD)
                        }
                    }
                    0xb9 => {
                        // UD r/m/ reg
                        context.modrm_rw_sub3(is, |_, _, _| Ok(IrOp::UD), |_, _, _| Ok(IrOp::UD))
                    }
                    0xba => {
                        // grp8 r/m16/32, imm8
                        context.modrm_rv_sub3(
                            is,
                            |is, s, c| {
                                let sub_op = OpGrp8::from_idx3(c);
                                let imm = is.fetch_u8()?;
                                match sub_op {
                                    OpGrp8::BT => Ok(IrOp::BT_Rw_Ib(s, imm)),
                                    OpGrp8::BTS => Ok(IrOp::BTS_Rw_Ib(s, imm)),
                                    OpGrp8::BTR => Ok(IrOp::BTR_Rw_Ib(s, imm)),
                                    OpGrp8::BTC => Ok(IrOp::BTC_Rw_Ib(s, imm)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                            |is, s, c| {
                                let sub_op = OpGrp8::from_idx3(c);
                                let imm = is.fetch_u8()?;
                                match sub_op {
                                    OpGrp8::BT => Ok(IrOp::BT_Mw_Ib(s, imm)),
                                    OpGrp8::BTS => Ok(IrOp::BTS_Mw_Ib(s, imm)),
                                    OpGrp8::BTR => Ok(IrOp::BTR_Mw_Ib(s, imm)),
                                    OpGrp8::BTC => Ok(IrOp::BTC_Mw_Ib(s, imm)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                            |is, s, c| {
                                let sub_op = OpGrp8::from_idx3(c);
                                let imm = is.fetch_u8()?;
                                match sub_op {
                                    OpGrp8::BT => Ok(IrOp::BT_Rd_Ib(s, imm)),
                                    OpGrp8::BTS => Ok(IrOp::BTS_Rd_Ib(s, imm)),
                                    OpGrp8::BTR => Ok(IrOp::BTR_Rd_Ib(s, imm)),
                                    OpGrp8::BTC => Ok(IrOp::BTC_Rd_Ib(s, imm)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                            |is, s, c| {
                                let sub_op = OpGrp8::from_idx3(c);
                                let imm = is.fetch_u8()?;
                                match sub_op {
                                    OpGrp8::BT => Ok(IrOp::BT_Md_Ib(s, imm)),
                                    OpGrp8::BTS => Ok(IrOp::BTS_Md_Ib(s, imm)),
                                    OpGrp8::BTR => Ok(IrOp::BTR_Md_Ib(s, imm)),
                                    OpGrp8::BTC => Ok(IrOp::BTC_Md_Ib(s, imm)),
                                    _ => Ok(IrOp::UD),
                                }
                            },
                        )
                    }
                    0xbb => {
                        // BTC r/m16/32, r16/32
                        context.modrm_rv(
                            is,
                            IrOp::BTC_Rw_Rw,
                            IrOp::BTC_Mw_Rw,
                            IrOp::BTC_Rd_Rd,
                            IrOp::BTC_Md_Rd,
                        )
                    }
                    0xbc => {
                        // BSF r16/32, r/m16/32
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::BSF_Rw_Rw(r, m),
                            |m, r| IrOp::BSF_Rw_Mw(r, m),
                            |m, r| IrOp::BSF_Rd_Rd(r, m),
                            |m, r| IrOp::BSF_Rd_Md(r, m),
                        )
                    }
                    0xbd => {
                        // BSR r16/32, r/m16/32
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::BSR_Rw_Rw(r, m),
                            |m, r| IrOp::BSR_Rw_Mw(r, m),
                            |m, r| IrOp::BSR_Rd_Rd(r, m),
                            |m, r| IrOp::BSR_Rd_Md(r, m),
                        )
                    }
                    0xbe => {
                        // MOVSX r16/32, r/m8
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::MOVSX_Rw_Rb(r, GprIndex8::from_u8(m as u8)),
                            |m, r| IrOp::MOVSX_Rw_Mb(r, m),
                            |m, r| IrOp::MOVSX_Rd_Rb(r, GprIndex8::from_u8(m as u8)),
                            |m, r| IrOp::MOVSX_Rd_Mb(r, m),
                        )
                    }
                    0xbf => {
                        // MOVSX r16/32, r/m16
                        context.modrm_rv(
                            is,
                            |m, r| IrOp::MOV_Rw_Rw(r, m),
                            |m, r| IrOp::MOV_Rw_Mw(r, m),
                            |m, r| IrOp::MOVSX_Rd_Rw(r, m.downgrade()),
                            |m, r| IrOp::MOVSX_Rd_Mw(r, m),
                        )
                    }

                    0xc0 => {
                        // XADD r/m8, r8
                        context.modrm_rb(is, IrOp::XADD_Rb_Rb, IrOp::XADD_Mb_Rb)
                    }
                    0xc1 => {
                        // XADD r/m16/32, r16/32
                        context.modrm_rv(
                            is,
                            IrOp::XADD_Rw_Rw,
                            IrOp::XADD_Mw_Rw,
                            IrOp::XADD_Rd_Rd,
                            IrOp::XADD_Md_Rd,
                        )
                    }

                    0xc8..=0xcf => {
                        // BSWAP r32
                        let reg = GprIndex32::from_u8(second_opcode);
                        Ok(IrOp::BSWAP_Rd(reg))
                    }

                    0x10..=0x17 | 0x28..=0x2f | 0x50..=0x7f | 0xae | 0xc2..=0xc6 | 0xd0..=0xfe => {
                        // MMX/SSE/SSE2 (src/simd.csv). F2/F3 win over 66; with
                        // F2/F3, 66 stays an operand-size prefix and is ignored.
                        let prefix = match context.repeat_prefix {
                            RepeatPrefix::F2 => SimdPrefix::F2,
                            RepeatPrefix::F3 => SimdPrefix::F3,
                            RepeatPrefix::None if context.is_opsize32 != self.use32 => {
                                SimdPrefix::P66
                            }
                            RepeatPrefix::None => SimdPrefix::None,
                        };
                        context.decode_simd(is, second_opcode, prefix)
                    }

                    _ => {
                        // unimplemented!(
                        //     "Two-byte opcode {:#x} {:#x} is not implemented yet",
                        //     opcode,
                        //     second_opcode
                        // );
                        Ok(IrOp::UD)
                    }
                }
            }

            _ => {
                // unimplemented!("Opcode {:#x} is not implemented yet", opcode);
                Ok(IrOp::UD)
            }
        }
    }
}

impl DecoderContext {
    /// Creates a new `DecoderContext` with default settings.
    #[inline]
    pub const fn new(use32: bool) -> Self {
        Self {
            segment_override: None,
            is_opsize32: use32,
            is_addr32: use32,
            repeat_prefix: RepeatPrefix::None,
        }
    }

    /// Fetches a direct memory offset (moffs) operand in the current address size.
    #[inline]
    pub fn fetch_moffs<E>(&self, is: &mut impl Fetch<E = E>) -> Result<MemOpr, E> {
        let segment = self.segment_override.unwrap_or(DS);
        if self.is_addr32 {
            let offset = Offset32(is.fetch_u32()?);
            Ok(MemOpr32::from_sel_off(segment, offset).into())
        } else {
            let offset = Offset16(is.fetch_u16()?);
            Ok(MemOpr16::from_sel_off(segment, offset).into())
        }
    }

    /// Decodes instructions with ModR/M byte and 8-bit register operand.
    #[inline]
    pub fn modrm_rb<E>(
        &self,
        is: &mut impl Fetch<E = E>,
        ir_rb: impl Fn(GprIndex8, GprIndex8) -> IrOp,
        ir_mb: impl Fn(MemOpr, GprIndex8) -> IrOp,
    ) -> Result<IrOp, E> {
        self.modrm_rb_sub3(
            is,
            |_, s, c| Ok(ir_rb(s, GprIndex8::from_idx3(c))),
            |_, s, c| Ok(ir_mb(s, GprIndex8::from_idx3(c))),
        )
    }

    /// Decodes instructions with ModR/M byte and reg field indicates a subcode
    #[inline]
    pub fn modrm_rb_sub3<IS, E>(
        &self,
        is: &mut IS,
        ir_rb: impl Fn(&mut IS, GprIndex8, Index3) -> Result<IrOp, E>,
        ir_mb: impl Fn(&mut IS, MemOpr, Index3) -> Result<IrOp, E>,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_mw_sub3(
            is,
            |is, modrm| {
                ir_rb(
                    is,
                    GprIndex8::from_idx3(modrm.rm_index()),
                    modrm.reg_index(),
                )
            },
            ir_mb,
        )
    }

    /// Decodes instructions with ModR/M byte and register operand whose size depends on the operand-size override prefix.
    #[inline]
    pub fn modrm_rv<E>(
        &self,
        is: &mut impl Fetch<E = E>,
        ir_rw: impl Fn(GprIndex16, GprIndex16) -> IrOp,
        ir_mw: impl Fn(MemOpr, GprIndex16) -> IrOp,
        ir_rd: impl Fn(GprIndex32, GprIndex32) -> IrOp,
        ir_md: impl Fn(MemOpr, GprIndex32) -> IrOp,
    ) -> Result<IrOp, E> {
        self.modrm_rv_sub3(
            is,
            |_, s, c| Ok(ir_rw(s, GprIndex16::from_idx3(c))),
            |_, s, c| Ok(ir_mw(s, GprIndex16::from_idx3(c))),
            |_, s, c| Ok(ir_rd(s, GprIndex32::from_idx3(c))),
            |_, s, c| Ok(ir_md(s, GprIndex32::from_idx3(c))),
        )
    }

    /// Decodes instructions with ModR/M byte and register operand whose size depends on the operand-size override prefix, and an immediate 8-bit value.
    #[inline]
    pub fn modrm_rv_ib<IS, E>(
        &self,
        is: &mut IS,
        ir_rw: impl Fn(GprIndex16, GprIndex16, u8) -> IrOp,
        ir_mw: impl Fn(MemOpr, GprIndex16, u8) -> IrOp,
        ir_rd: impl Fn(GprIndex32, GprIndex32, u8) -> IrOp,
        ir_md: impl Fn(MemOpr, GprIndex32, u8) -> IrOp,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_rv_sub3(
            is,
            |is, s, c| {
                let imm = is.fetch_u8()?;
                Ok(ir_rw(s, GprIndex16::from_idx3(c), imm))
            },
            |is, s, c| {
                let imm = is.fetch_u8()?;
                Ok(ir_mw(s, GprIndex16::from_idx3(c), imm))
            },
            |is, s, c| {
                let imm = is.fetch_u8()?;
                Ok(ir_rd(s, GprIndex32::from_idx3(c), imm))
            },
            |is, s, c| {
                let imm = is.fetch_u8()?;
                Ok(ir_md(s, GprIndex32::from_idx3(c), imm))
            },
        )
    }

    /// Decodes instructions with ModR/M byte and register operand whose size depends on the operand-size override prefix, and reg field indicates a subcode
    #[inline]
    pub fn modrm_rv_sub3<IS, E>(
        &self,
        is: &mut IS,
        ir_rw: impl Fn(&mut IS, GprIndex16, Index3) -> Result<IrOp, E>,
        ir_mw: impl Fn(&mut IS, MemOpr, Index3) -> Result<IrOp, E>,
        ir_rd: impl Fn(&mut IS, GprIndex32, Index3) -> Result<IrOp, E>,
        ir_md: impl Fn(&mut IS, MemOpr, Index3) -> Result<IrOp, E>,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_mv_sub3(
            is,
            |s, modrm| {
                let reg_index = modrm.reg_index();
                let rm = modrm.rm_index();
                if self.is_opsize32 {
                    ir_rd(s, GprIndex32::from_idx3(rm), reg_index)
                } else {
                    ir_rw(s, GprIndex16::from_idx3(rm), reg_index)
                }
            },
            ir_mw,
            ir_md,
        )
    }

    /// Decodes instructions with ModR/M byte and 16-bit register operand.
    #[inline]
    pub fn modrm_rw<IS, E>(
        &self,
        is: &mut IS,
        ir_rw: impl Fn(GprIndex16, GprIndex16) -> IrOp,
        ir_mw: impl Fn(MemOpr, GprIndex16) -> IrOp,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_rw_sub3(
            is,
            |_, s, c| Ok(ir_rw(s, GprIndex16::from_idx3(c))),
            |_, s, c| Ok(ir_mw(s, GprIndex16::from_idx3(c))),
        )
    }

    /// Decodes instructions with ModR/M byte and 16-bit register operand, and reg field indicates a subcode
    #[inline]
    pub fn modrm_rw_sub3<IS, E>(
        &self,
        is: &mut IS,
        ir_rw: impl Fn(&mut IS, GprIndex16, Index3) -> Result<IrOp, E>,
        ir_mw: impl Fn(&mut IS, MemOpr, Index3) -> Result<IrOp, E>,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_mw_sub3(
            is,
            |is, modrm| {
                let reg_index = modrm.reg_index();
                let rm = modrm.rm_index();
                ir_rw(is, GprIndex16::from_idx3(rm), reg_index)
            },
            ir_mw,
        )
    }

    /// Decodes instructions with ModR/M byte and memory operand whose size depends on the operand-size override prefix
    #[inline]
    pub fn modrm_mv<IS, E>(
        &self,
        is: &mut IS,
        ir_rv: impl Fn(ModRM) -> IrOp,
        ir_mw: impl Fn(MemOpr, GprIndex16) -> IrOp,
        ir_md: impl Fn(MemOpr, GprIndex32) -> IrOp,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_mv_sub3(
            is,
            |_, modrm| Ok(ir_rv(modrm)),
            |_, s, c| Ok(ir_mw(s, GprIndex16::from_idx3(c))),
            |_, s, c| Ok(ir_md(s, GprIndex32::from_idx3(c))),
        )
    }

    /// Decodes instructions with ModR/M byte and memory operand whose size depends on the operand-size override prefix, and reg field indicates a subcode
    #[inline]
    pub fn modrm_mv_sub3<IS, E>(
        &self,
        is: &mut IS,
        ir_rv: impl Fn(&mut IS, ModRM) -> Result<IrOp, E>,
        ir_mw: impl Fn(&mut IS, MemOpr, Index3) -> Result<IrOp, E>,
        ir_md: impl Fn(&mut IS, MemOpr, Index3) -> Result<IrOp, E>,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        self.modrm_mw_sub3(is, ir_rv, |is, mem, index| {
            if self.is_opsize32 {
                ir_md(is, mem, index)
            } else {
                ir_mw(is, mem, index)
            }
        })
    }

    /// Decodes `0F opcode` in the SIMD ranges. ModR/M, SIB, displacement and
    /// imm8 are read whenever the opcode has them, even when the prefix or
    /// ModR/M selects no form and the result is `UD`.
    fn decode_simd<IS, E>(&self, is: &mut IS, opcode: u8, prefix: SimdPrefix) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        let shape = simd_shape(opcode);
        if !shape.has_modrm {
            return Ok(match simd_lookup(opcode, prefix, Index3::I000, false) {
                Some(op) => IrOp::SIMD(op),
                None => IrOp::UD,
            });
        }
        self.modrm_mw_sub3(
            is,
            |is, modrm| {
                let imm = if shape.has_imm8 { is.fetch_u8()? } else { 0 };
                let Some(op) = simd_lookup(opcode, prefix, modrm.reg_index(), false) else {
                    return Ok(IrOp::UD);
                };
                let reg = SimdReg::new(op.reg_kind(), modrm.reg_index());
                let rm = SimdReg::new(op.rm_kind(), modrm.rm_index());
                Ok(if op.implicit_di() {
                    let di = StringOpr {
                        segment: self.segment_override.unwrap_or(DS),
                        is_addr32: self.is_addr32,
                    };
                    IrOp::SIMD_S(op, reg, rm, di)
                } else if matches!(rm, SimdReg::None) {
                    IrOp::SIMD(op)
                } else {
                    IrOp::SIMD_R(op, reg, rm, imm)
                })
            },
            |is, mem, reg_index| {
                let imm = if shape.has_imm8 { is.fetch_u8()? } else { 0 };
                let Some(op) = simd_lookup(opcode, prefix, reg_index, true) else {
                    return Ok(IrOp::UD);
                };
                let reg = SimdReg::new(op.reg_kind(), reg_index);
                Ok(IrOp::SIMD_M(op, reg, mem, imm))
            },
        )
    }

    /// Decodes instructions with ModR/M byte and memory operand in the current address size.
    #[inline]
    pub fn modrm_mw_sub3<IS, E>(
        &self,
        is: &mut IS,
        ir_rw: impl Fn(&mut IS, ModRM) -> Result<IrOp, E>,
        ir_mw: impl Fn(&mut IS, MemOpr, Index3) -> Result<IrOp, E>,
    ) -> Result<IrOp, E>
    where
        IS: Fetch<E = E>,
    {
        if self.is_addr32 {
            let modrm = ModRm32::fetch(is, self.segment_override)?;
            match modrm.reg_or_mem() {
                RegOrMem::Reg(_) => ir_rw(is, modrm.raw_modrm()),
                RegOrMem::Mem(mem) => ir_mw(is, MemOpr::A32(mem), modrm.reg_index()),
            }
        } else {
            let modrm = ModRm16::fetch(is, self.segment_override)?;
            match modrm.reg_or_mem() {
                RegOrMem::Reg(_) => ir_rw(is, modrm.raw_modrm()),
                RegOrMem::Mem(mem) => ir_mw(is, MemOpr::A16(mem), modrm.reg_index()),
            }
        }
    }
}
