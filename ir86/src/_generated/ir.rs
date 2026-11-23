// This file is automatically @generated at build time. DO NOT EDIT DIRECTLY.

use crate::prelude::*;

/// Intermediate Representation for x86 opcodes
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrOp {
    /// `AAA`
    AAA,
    /// `AAD imm8`
    AAD_Ib(u8),
    /// `AAM imm8`
    AAM_Ib(u8),
    /// `AAS`
    AAS,
    /// `ADC [mem8], imm8`
    ADC_Mb_Ib(MemOpr, u8),
    /// `ADC [mem8], reg8`
    ADC_Mb_Rb(MemOpr, GprIndex8),
    /// `ADC [mem32], imm32`
    ADC_Md_Id(MemOpr, u32),
    /// `ADC [mem32], reg32`
    ADC_Md_Rd(MemOpr, GprIndex32),
    /// `ADC [mem16], imm16`
    ADC_Mw_Iw(MemOpr, u16),
    /// `ADC [mem16], reg16`
    ADC_Mw_Rw(MemOpr, GprIndex16),
    /// `ADC reg8, imm8`
    ADC_Rb_Ib(GprIndex8, u8),
    /// `ADC reg8, [mem8]`
    ADC_Rb_Mb(GprIndex8, MemOpr),
    /// `ADC reg8, reg8`
    ADC_Rb_Rb(GprIndex8, GprIndex8),
    /// `ADC reg32, imm32`
    ADC_Rd_Id(GprIndex32, u32),
    /// `ADC reg32, [mem32]`
    ADC_Rd_Md(GprIndex32, MemOpr),
    /// `ADC reg32, reg32`
    ADC_Rd_Rd(GprIndex32, GprIndex32),
    /// `ADC reg16, imm16`
    ADC_Rw_Iw(GprIndex16, u16),
    /// `ADC reg16, [mem16]`
    ADC_Rw_Mw(GprIndex16, MemOpr),
    /// `ADC reg16, reg16`
    ADC_Rw_Rw(GprIndex16, GprIndex16),
    /// `ADD [mem8], imm8`
    ADD_Mb_Ib(MemOpr, u8),
    /// `ADD [mem8], reg8`
    ADD_Mb_Rb(MemOpr, GprIndex8),
    /// `ADD [mem32], imm32`
    ADD_Md_Id(MemOpr, u32),
    /// `ADD [mem32], reg32`
    ADD_Md_Rd(MemOpr, GprIndex32),
    /// `ADD [mem16], imm16`
    ADD_Mw_Iw(MemOpr, u16),
    /// `ADD [mem16], reg16`
    ADD_Mw_Rw(MemOpr, GprIndex16),
    /// `ADD reg8, imm8`
    ADD_Rb_Ib(GprIndex8, u8),
    /// `ADD reg8, [mem8]`
    ADD_Rb_Mb(GprIndex8, MemOpr),
    /// `ADD reg8, reg8`
    ADD_Rb_Rb(GprIndex8, GprIndex8),
    /// `ADD reg32, imm32`
    ADD_Rd_Id(GprIndex32, u32),
    /// `ADD reg32, [mem32]`
    ADD_Rd_Md(GprIndex32, MemOpr),
    /// `ADD reg32, reg32`
    ADD_Rd_Rd(GprIndex32, GprIndex32),
    /// `ADD reg16, imm16`
    ADD_Rw_Iw(GprIndex16, u16),
    /// `ADD reg16, [mem16]`
    ADD_Rw_Mw(GprIndex16, MemOpr),
    /// `ADD reg16, reg16`
    ADD_Rw_Rw(GprIndex16, GprIndex16),
    /// `AND [mem8], imm8`
    AND_Mb_Ib(MemOpr, u8),
    /// `AND [mem8], reg8`
    AND_Mb_Rb(MemOpr, GprIndex8),
    /// `AND [mem32], imm32`
    AND_Md_Id(MemOpr, u32),
    /// `AND [mem32], reg32`
    AND_Md_Rd(MemOpr, GprIndex32),
    /// `AND [mem16], imm16`
    AND_Mw_Iw(MemOpr, u16),
    /// `AND [mem16], reg16`
    AND_Mw_Rw(MemOpr, GprIndex16),
    /// `AND reg8, imm8`
    AND_Rb_Ib(GprIndex8, u8),
    /// `AND reg8, [mem8]`
    AND_Rb_Mb(GprIndex8, MemOpr),
    /// `AND reg8, reg8`
    AND_Rb_Rb(GprIndex8, GprIndex8),
    /// `AND reg32, imm32`
    AND_Rd_Id(GprIndex32, u32),
    /// `AND reg32, [mem32]`
    AND_Rd_Md(GprIndex32, MemOpr),
    /// `AND reg32, reg32`
    AND_Rd_Rd(GprIndex32, GprIndex32),
    /// `AND reg16, imm16`
    AND_Rw_Iw(GprIndex16, u16),
    /// `AND reg16, [mem16]`
    AND_Rw_Mw(GprIndex16, MemOpr),
    /// `AND reg16, reg16`
    AND_Rw_Rw(GprIndex16, GprIndex16),
    /// `ARPL [mem16], reg16`
    ARPL_Mw_Rw(MemOpr, GprIndex16),
    /// `ARPL reg16, reg16`
    ARPL_Rw_Rw(GprIndex16, GprIndex16),
    /// `BOUND reg32, [mem32]`
    BOUND_Rd_Md(GprIndex32, MemOpr),
    /// `BOUND reg16, [mem16]`
    BOUND_Rw_Mw(GprIndex16, MemOpr),
    /// `BSF reg32, [mem32]`
    BSF_Rd_Md(GprIndex32, MemOpr),
    /// `BSF reg32, reg32`
    BSF_Rd_Rd(GprIndex32, GprIndex32),
    /// `BSF reg16, [mem16]`
    BSF_Rw_Mw(GprIndex16, MemOpr),
    /// `BSF reg16, reg16`
    BSF_Rw_Rw(GprIndex16, GprIndex16),
    /// `BSR reg32, [mem32]`
    BSR_Rd_Md(GprIndex32, MemOpr),
    /// `BSR reg32, reg32`
    BSR_Rd_Rd(GprIndex32, GprIndex32),
    /// `BSR reg16, [mem16]`
    BSR_Rw_Mw(GprIndex16, MemOpr),
    /// `BSR reg16, reg16`
    BSR_Rw_Rw(GprIndex16, GprIndex16),
    /// `BSWAP reg32`
    BSWAP_Rd(GprIndex32),
    /// `BTC [mem32], imm8`
    BTC_Md_Ib(MemOpr, u8),
    /// `BTC [mem32], reg32`
    BTC_Md_Rd(MemOpr, GprIndex32),
    /// `BTC [mem16], imm8`
    BTC_Mw_Ib(MemOpr, u8),
    /// `BTC [mem16], reg16`
    BTC_Mw_Rw(MemOpr, GprIndex16),
    /// `BTC reg32, imm8`
    BTC_Rd_Ib(GprIndex32, u8),
    /// `BTC reg32, reg32`
    BTC_Rd_Rd(GprIndex32, GprIndex32),
    /// `BTC reg16, imm8`
    BTC_Rw_Ib(GprIndex16, u8),
    /// `BTC reg16, reg16`
    BTC_Rw_Rw(GprIndex16, GprIndex16),
    /// `BTR [mem32], imm8`
    BTR_Md_Ib(MemOpr, u8),
    /// `BTR [mem32], reg32`
    BTR_Md_Rd(MemOpr, GprIndex32),
    /// `BTR [mem16], imm8`
    BTR_Mw_Ib(MemOpr, u8),
    /// `BTR [mem16], reg16`
    BTR_Mw_Rw(MemOpr, GprIndex16),
    /// `BTR reg32, imm8`
    BTR_Rd_Ib(GprIndex32, u8),
    /// `BTR reg32, reg32`
    BTR_Rd_Rd(GprIndex32, GprIndex32),
    /// `BTR reg16, imm8`
    BTR_Rw_Ib(GprIndex16, u8),
    /// `BTR reg16, reg16`
    BTR_Rw_Rw(GprIndex16, GprIndex16),
    /// `BTS [mem32], imm8`
    BTS_Md_Ib(MemOpr, u8),
    /// `BTS [mem32], reg32`
    BTS_Md_Rd(MemOpr, GprIndex32),
    /// `BTS [mem16], imm8`
    BTS_Mw_Ib(MemOpr, u8),
    /// `BTS [mem16], reg16`
    BTS_Mw_Rw(MemOpr, GprIndex16),
    /// `BTS reg32, imm8`
    BTS_Rd_Ib(GprIndex32, u8),
    /// `BTS reg32, reg32`
    BTS_Rd_Rd(GprIndex32, GprIndex32),
    /// `BTS reg16, imm8`
    BTS_Rw_Ib(GprIndex16, u8),
    /// `BTS reg16, reg16`
    BTS_Rw_Rw(GprIndex16, GprIndex16),
    /// `BT [mem32], imm8`
    BT_Md_Ib(MemOpr, u8),
    /// `BT [mem32], reg32`
    BT_Md_Rd(MemOpr, GprIndex32),
    /// `BT [mem16], imm8`
    BT_Mw_Ib(MemOpr, u8),
    /// `BT [mem16], reg16`
    BT_Mw_Rw(MemOpr, GprIndex16),
    /// `BT reg32, imm8`
    BT_Rd_Ib(GprIndex32, u8),
    /// `BT reg32, reg32`
    BT_Rd_Rd(GprIndex32, GprIndex32),
    /// `BT reg16, imm8`
    BT_Rw_Ib(GprIndex16, u8),
    /// `BT reg16, reg16`
    BT_Rw_Rw(GprIndex16, GprIndex16),
    /// `o16: CALLF a16:16`
    CALLF_ApO16(SegmentSelector, Offset16),
    /// `o32: CALLF a16:32`
    CALLF_ApO32(SegmentSelector, Offset32),
    /// `o16: CALLF [mem]`
    CALLF_MpO16(MemOpr),
    /// `o32: CALLF [mem]`
    CALLF_MpO32(MemOpr),
    /// `CALL Offset32`
    CALL_Jd(Offset32),
    /// `CALL Offset16`
    CALL_Jw(Offset16),
    /// `CALL [mem32]`
    CALL_Md(MemOpr),
    /// `CALL [mem16]`
    CALL_Mw(MemOpr),
    /// `CALL reg32`
    CALL_Rd(GprIndex32),
    /// `CALL reg16`
    CALL_Rw(GprIndex16),
    /// `CDQ`
    CDQ,
    /// `CLC`
    CLC,
    /// `CLD`
    CLD,
    /// `CLI`
    CLI,
    /// `CLTS`
    CLTS,
    /// `CMC`
    CMC,
    /// `CMOV cc, reg32, [mem32]`
    CMOV_Rd_Md(CC, GprIndex32, MemOpr),
    /// `CMOV cc, reg32, reg32`
    CMOV_Rd_Rd(CC, GprIndex32, GprIndex32),
    /// `CMOV cc, reg16, [mem16]`
    CMOV_Rw_Mw(CC, GprIndex16, MemOpr),
    /// `CMOV cc, reg16, reg16`
    CMOV_Rw_Rw(CC, GprIndex16, GprIndex16),
    /// `CMPSb (StringOpr)`
    CMPSB(StringOpr),
    /// `CMPSd (StringOpr)`
    CMPSD(StringOpr),
    /// `CMPSw (StringOpr)`
    CMPSW(StringOpr),
    /// `CMPXCHG [mem8], reg8`
    CMPXCHG_Mb_Rb(MemOpr, GprIndex8),
    /// `CMPXCHG [mem32], reg32`
    CMPXCHG_Md_Rd(MemOpr, GprIndex32),
    /// `CMPXCHG [mem16], reg16`
    CMPXCHG_Mw_Rw(MemOpr, GprIndex16),
    /// `CMPXCHG reg8, reg8`
    CMPXCHG_Rb_Rb(GprIndex8, GprIndex8),
    /// `CMPXCHG reg32, reg32`
    CMPXCHG_Rd_Rd(GprIndex32, GprIndex32),
    /// `CMPXCHG reg16, reg16`
    CMPXCHG_Rw_Rw(GprIndex16, GprIndex16),
    /// `CMP [mem8], imm8`
    CMP_Mb_Ib(MemOpr, u8),
    /// `CMP [mem8], reg8`
    CMP_Mb_Rb(MemOpr, GprIndex8),
    /// `CMP [mem32], imm32`
    CMP_Md_Id(MemOpr, u32),
    /// `CMP [mem32], reg32`
    CMP_Md_Rd(MemOpr, GprIndex32),
    /// `CMP [mem16], imm16`
    CMP_Mw_Iw(MemOpr, u16),
    /// `CMP [mem16], reg16`
    CMP_Mw_Rw(MemOpr, GprIndex16),
    /// `CMP reg8, imm8`
    CMP_Rb_Ib(GprIndex8, u8),
    /// `CMP reg8, [mem8]`
    CMP_Rb_Mb(GprIndex8, MemOpr),
    /// `CMP reg8, reg8`
    CMP_Rb_Rb(GprIndex8, GprIndex8),
    /// `CMP reg32, imm32`
    CMP_Rd_Id(GprIndex32, u32),
    /// `CMP reg32, [mem32]`
    CMP_Rd_Md(GprIndex32, MemOpr),
    /// `CMP reg32, reg32`
    CMP_Rd_Rd(GprIndex32, GprIndex32),
    /// `CMP reg16, imm16`
    CMP_Rw_Iw(GprIndex16, u16),
    /// `CMP reg16, [mem16]`
    CMP_Rw_Mw(GprIndex16, MemOpr),
    /// `CMP reg16, reg16`
    CMP_Rw_Rw(GprIndex16, GprIndex16),
    /// `CPUID`
    CPUID,
    /// `CWD`
    CWD,
    /// `DAA`
    DAA,
    /// `DAS`
    DAS,
    /// `DEC [mem8]`
    DEC_Mb(MemOpr),
    /// `DEC [mem32]`
    DEC_Md(MemOpr),
    /// `DEC [mem16]`
    DEC_Mw(MemOpr),
    /// `DEC reg8`
    DEC_Rb(GprIndex8),
    /// `DEC reg32`
    DEC_Rd(GprIndex32),
    /// `DEC reg16`
    DEC_Rw(GprIndex16),
    /// `DIV [mem8]`
    DIV_Mb(MemOpr),
    /// `DIV [mem32]`
    DIV_Md(MemOpr),
    /// `DIV [mem16]`
    DIV_Mw(MemOpr),
    /// `DIV reg8`
    DIV_Rb(GprIndex8),
    /// `DIV reg32`
    DIV_Rd(GprIndex32),
    /// `DIV reg16`
    DIV_Rw(GprIndex16),
    /// `ENTER imm16, imm8`
    ENTER_Iw_Ib(u16, u8),
    /// `esc op3, reg3, [mem8]`
    ESC_Mb(EscOpcode, u8, MemOpr),
    /// `esc op3, reg3, reg8`
    ESC_Rb(EscOpcode, u8, GprIndex8),
    /// `HLT`
    HLT,
    /// `ICEBP`
    ICEBP,
    /// `IDIV [mem8]`
    IDIV_Mb(MemOpr),
    /// `IDIV [mem32]`
    IDIV_Md(MemOpr),
    /// `IDIV [mem16]`
    IDIV_Mw(MemOpr),
    /// `IDIV reg8`
    IDIV_Rb(GprIndex8),
    /// `IDIV reg32`
    IDIV_Rd(GprIndex32),
    /// `IDIV reg16`
    IDIV_Rw(GprIndex16),
    /// `IMUL [mem8]`
    IMUL_Mb(MemOpr),
    /// `IMUL [mem32]`
    IMUL_Md(MemOpr),
    /// `IMUL [mem16]`
    IMUL_Mw(MemOpr),
    /// `IMUL reg8`
    IMUL_Rb(GprIndex8),
    /// `IMUL reg32`
    IMUL_Rd(GprIndex32),
    /// `IMUL reg32, [mem32]`
    IMUL_Rd_Md(GprIndex32, MemOpr),
    /// `IMUL reg32, [mem32], imm32`
    IMUL_Rd_Md_Id(GprIndex32, MemOpr, u32),
    /// `IMUL reg32, reg32`
    IMUL_Rd_Rd(GprIndex32, GprIndex32),
    /// `IMUL reg32, reg32, imm32`
    IMUL_Rd_Rd_Id(GprIndex32, GprIndex32, u32),
    /// `IMUL reg16`
    IMUL_Rw(GprIndex16),
    /// `IMUL reg16, [mem16]`
    IMUL_Rw_Mw(GprIndex16, MemOpr),
    /// `IMUL reg16, [mem16], imm16`
    IMUL_Rw_Mw_Iw(GprIndex16, MemOpr, u16),
    /// `IMUL reg16, reg16`
    IMUL_Rw_Rw(GprIndex16, GprIndex16),
    /// `IMUL reg16, reg16, imm16`
    IMUL_Rw_Rw_Iw(GprIndex16, GprIndex16, u16),
    /// `INC [mem8]`
    INC_Mb(MemOpr),
    /// `INC [mem32]`
    INC_Md(MemOpr),
    /// `INC [mem16]`
    INC_Mw(MemOpr),
    /// `INC reg8`
    INC_Rb(GprIndex8),
    /// `INC reg32`
    INC_Rd(GprIndex32),
    /// `INC reg16`
    INC_Rw(GprIndex16),
    /// `INSb (StringOprEs)`
    INSB(StringOprEs),
    /// `INSd (StringOprEs)`
    INSD(StringOprEs),
    /// `INSw (StringOprEs)`
    INSW(StringOprEs),
    /// `INT3`
    INT3,
    /// `INTO`
    INTO,
    /// `INT imm8`
    INT_Ib(u8),
    /// `INVD`
    INVD,
    /// `INVLPG [mem8]`
    INVLPG_Mb(MemOpr),
    /// `INVLPG [mem32]`
    INVLPG_Md(MemOpr),
    /// `INVLPG [mem16]`
    INVLPG_Mw(MemOpr),
    /// `INVLPG reg8`
    INVLPG_Rb(GprIndex8),
    /// `INVLPG reg32`
    INVLPG_Rd(GprIndex32),
    /// `INVLPG reg16`
    INVLPG_Rw(GprIndex16),
    /// `IN eax, dx`
    IN_Ad_Dx,
    /// `IN eax, imm8`
    IN_Ad_Ib(u8),
    /// `IN al, dx`
    IN_Al_Dx,
    /// `IN al, imm8`
    IN_Al_Ib(u8),
    /// `IN ax, dx`
    IN_Aw_Dx,
    /// `IN ax, imm8`
    IN_Aw_Ib(u8),
    /// `IRET`
    IRET,
    /// `IRETD`
    IRETD,
    /// `JCC Offset32`
    JCC_Jd(CC, Offset32),
    /// `JCC Offset16`
    JCC_Jw(CC, Offset16),
    /// `JCXZ Offset32`
    JCXZ_Jd(Offset32),
    /// `JCXZ Offset16`
    JCXZ_Jw(Offset16),
    /// `JECXZ Offset32`
    JECXZ_Jd(Offset32),
    /// `JECXZ Offset16`
    JECXZ_Jw(Offset16),
    /// `o16: JMPF a16:16`
    JMPF_ApO16(SegmentSelector, Offset16),
    /// `o32: JMPF a16:32`
    JMPF_ApO32(SegmentSelector, Offset32),
    /// `o16: JMPF [mem]`
    JMPF_MpO16(MemOpr),
    /// `o32: JMPF [mem]`
    JMPF_MpO32(MemOpr),
    /// `JMP Offset32`
    JMP_Jd(Offset32),
    /// `JMP Offset16`
    JMP_Jw(Offset16),
    /// `JMP [mem32]`
    JMP_Md(MemOpr),
    /// `JMP [mem16]`
    JMP_Mw(MemOpr),
    /// `JMP reg32`
    JMP_Rd(GprIndex32),
    /// `JMP reg16`
    JMP_Rw(GprIndex16),
    /// `LAHF`
    LAHF,
    /// `LAR reg32, [mem16]`
    LAR_Rd_Mw(GprIndex32, MemOpr),
    /// `LAR reg32, reg16`
    LAR_Rd_Rw(GprIndex32, GprIndex16),
    /// `LAR reg16, [mem16]`
    LAR_Rw_Mw(GprIndex16, MemOpr),
    /// `LAR reg16, reg16`
    LAR_Rw_Rw(GprIndex16, GprIndex16),
    /// `LDS reg32, [mem32]`
    LDS_Rd_Md(GprIndex32, MemOpr),
    /// `LDS reg16, [mem16]`
    LDS_Rw_Mw(GprIndex16, MemOpr),
    /// `LEAVE`
    LEAVE,
    /// `LEA reg32, [mem32]`
    LEA_Rd_Md(GprIndex32, MemOpr),
    /// `LEA reg16, [mem16]`
    LEA_Rw_Mw(GprIndex16, MemOpr),
    /// `LES reg32, [mem32]`
    LES_Rd_Md(GprIndex32, MemOpr),
    /// `LES reg16, [mem16]`
    LES_Rw_Mw(GprIndex16, MemOpr),
    /// `LFS reg32, [mem32]`
    LFS_Rd_Md(GprIndex32, MemOpr),
    /// `LFS reg16, [mem16]`
    LFS_Rw_Mw(GprIndex16, MemOpr),
    /// `o16: LGDT [mem]`
    LGDT_MpO16(MemOpr),
    /// `o32: LGDT [mem]`
    LGDT_MpO32(MemOpr),
    /// `LGS reg32, [mem32]`
    LGS_Rd_Md(GprIndex32, MemOpr),
    /// `LGS reg16, [mem16]`
    LGS_Rw_Mw(GprIndex16, MemOpr),
    /// `o16: LIDT [mem]`
    LIDT_MpO16(MemOpr),
    /// `o32: LIDT [mem]`
    LIDT_MpO32(MemOpr),
    /// `LLDT [mem16]`
    LLDT_Mw(MemOpr),
    /// `LLDT reg16`
    LLDT_Rw(GprIndex16),
    /// `LMSW [mem16]`
    LMSW_Mw(MemOpr),
    /// `LMSW reg16`
    LMSW_Rw(GprIndex16),
    /// `LODSb (StringOpr)`
    LODSB(StringOpr),
    /// `LODSd (StringOpr)`
    LODSD(StringOpr),
    /// `LODSw (StringOpr)`
    LODSW(StringOpr),
    /// `LOOPNZ addr32, Offset32`; addr32: true = ECX, false = CX
    LOOPNZ_Jd(bool, Offset32),
    /// `LOOPNZ addr32, Offset16`; addr32: true = ECX, false = CX
    LOOPNZ_Jw(bool, Offset16),
    /// `LOOPZ addr32, Offset32`; addr32: true = ECX, false = CX
    LOOPZ_Jd(bool, Offset32),
    /// `LOOPZ addr32, Offset16`; addr32: true = ECX, false = CX
    LOOPZ_Jw(bool, Offset16),
    /// `LOOP addr32, Offset32`; addr32: true = ECX, false = CX
    LOOP_Jd(bool, Offset32),
    /// `LOOP addr32, Offset16`; addr32: true = ECX, false = CX
    LOOP_Jw(bool, Offset16),
    /// `LSL reg32, [mem16]`
    LSL_Rd_Mw(GprIndex32, MemOpr),
    /// `LSL reg32, reg16`
    LSL_Rd_Rw(GprIndex32, GprIndex16),
    /// `LSL reg16, [mem16]`
    LSL_Rw_Mw(GprIndex16, MemOpr),
    /// `LSL reg16, reg16`
    LSL_Rw_Rw(GprIndex16, GprIndex16),
    /// `LSS reg32, [mem32]`
    LSS_Rd_Md(GprIndex32, MemOpr),
    /// `LSS reg16, [mem16]`
    LSS_Rw_Mw(GprIndex16, MemOpr),
    /// `LTR [mem16]`
    LTR_Mw(MemOpr),
    /// `LTR reg16`
    LTR_Rw(GprIndex16),
    /// `MOVSb (StringOpr)`
    MOVSB(StringOpr),
    /// `MOVSd (StringOpr)`
    MOVSD(StringOpr),
    /// `MOVSw (StringOpr)`
    MOVSW(StringOpr),
    /// `MOVSX reg32, [mem8]`
    MOVSX_Rd_Mb(GprIndex32, MemOpr),
    /// `MOVSX reg32, [mem16]`
    MOVSX_Rd_Mw(GprIndex32, MemOpr),
    /// `MOVSX reg32, reg8`
    MOVSX_Rd_Rb(GprIndex32, GprIndex8),
    /// `MOVSX reg32, reg16`
    MOVSX_Rd_Rw(GprIndex32, GprIndex16),
    /// `MOVSX reg16, [mem8]`
    MOVSX_Rw_Mb(GprIndex16, MemOpr),
    /// `MOVSX reg16, reg8`
    MOVSX_Rw_Rb(GprIndex16, GprIndex8),
    /// `MOVZX reg32, [mem8]`
    MOVZX_Rd_Mb(GprIndex32, MemOpr),
    /// `MOVZX reg32, [mem16]`
    MOVZX_Rd_Mw(GprIndex32, MemOpr),
    /// `MOVZX reg32, reg8`
    MOVZX_Rd_Rb(GprIndex32, GprIndex8),
    /// `MOVZX reg32, reg16`
    MOVZX_Rd_Rw(GprIndex32, GprIndex16),
    /// `MOVZX reg16, [mem8]`
    MOVZX_Rw_Mb(GprIndex16, MemOpr),
    /// `MOVZX reg16, reg8`
    MOVZX_Rw_Rb(GprIndex16, GprIndex8),
    /// `MOV cr, reg32`
    MOV_Cr_Rd(CrIndex, GprIndex32),
    /// `MOV dr, reg32`
    MOV_Dr_Rd(DrIndex, GprIndex32),
    /// `MOV [mem8], imm8`
    MOV_Mb_Ib(MemOpr, u8),
    /// `MOV [mem8], reg8`
    MOV_Mb_Rb(MemOpr, GprIndex8),
    /// `MOV [mem32], imm32`
    MOV_Md_Id(MemOpr, u32),
    /// `MOV [mem32], reg32`
    MOV_Md_Rd(MemOpr, GprIndex32),
    /// `MOV [mem32], sr`
    MOV_Md_Sr(MemOpr, SrIndex),
    /// `MOV [mem16], imm16`
    MOV_Mw_Iw(MemOpr, u16),
    /// `MOV [mem16], reg16`
    MOV_Mw_Rw(MemOpr, GprIndex16),
    /// `MOV [mem16], sr`
    MOV_Mw_Sr(MemOpr, SrIndex),
    /// `MOV reg8, imm8`
    MOV_Rb_Ib(GprIndex8, u8),
    /// `MOV reg8, [mem8]`
    MOV_Rb_Mb(GprIndex8, MemOpr),
    /// `MOV reg8, reg8`
    MOV_Rb_Rb(GprIndex8, GprIndex8),
    /// `MOV reg32, cr`
    MOV_Rd_Cr(GprIndex32, CrIndex),
    /// `MOV reg32, dr`
    MOV_Rd_Dr(GprIndex32, DrIndex),
    /// `MOV reg32, imm32`
    MOV_Rd_Id(GprIndex32, u32),
    /// `MOV reg32, [mem32]`
    MOV_Rd_Md(GprIndex32, MemOpr),
    /// `MOV reg32, reg32`
    MOV_Rd_Rd(GprIndex32, GprIndex32),
    /// `MOV reg32, sr`
    MOV_Rd_Sr(GprIndex32, SrIndex),
    /// `MOV reg16, imm16`
    MOV_Rw_Iw(GprIndex16, u16),
    /// `MOV reg16, [mem16]`
    MOV_Rw_Mw(GprIndex16, MemOpr),
    /// `MOV reg16, reg16`
    MOV_Rw_Rw(GprIndex16, GprIndex16),
    /// `MOV reg16, sr`
    MOV_Rw_Sr(GprIndex16, SrIndex),
    /// `MOV sr, [mem32]`
    MOV_Sr_Md(SrIndex, MemOpr),
    /// `MOV sr, [mem16]`
    MOV_Sr_Mw(SrIndex, MemOpr),
    /// `MOV sr, reg32`
    MOV_Sr_Rd(SrIndex, GprIndex32),
    /// `MOV sr, reg16`
    MOV_Sr_Rw(SrIndex, GprIndex16),
    /// `MUL [mem8]`
    MUL_Mb(MemOpr),
    /// `MUL [mem32]`
    MUL_Md(MemOpr),
    /// `MUL [mem16]`
    MUL_Mw(MemOpr),
    /// `MUL reg8`
    MUL_Rb(GprIndex8),
    /// `MUL reg32`
    MUL_Rd(GprIndex32),
    /// `MUL reg16`
    MUL_Rw(GprIndex16),
    /// `NEG [mem8]`
    NEG_Mb(MemOpr),
    /// `NEG [mem32]`
    NEG_Md(MemOpr),
    /// `NEG [mem16]`
    NEG_Mw(MemOpr),
    /// `NEG reg8`
    NEG_Rb(GprIndex8),
    /// `NEG reg32`
    NEG_Rd(GprIndex32),
    /// `NEG reg16`
    NEG_Rw(GprIndex16),
    /// `NOP`
    NOP,
    /// `NOT [mem8]`
    NOT_Mb(MemOpr),
    /// `NOT [mem32]`
    NOT_Md(MemOpr),
    /// `NOT [mem16]`
    NOT_Mw(MemOpr),
    /// `NOT reg8`
    NOT_Rb(GprIndex8),
    /// `NOT reg32`
    NOT_Rd(GprIndex32),
    /// `NOT reg16`
    NOT_Rw(GprIndex16),
    /// `OR [mem8], imm8`
    OR_Mb_Ib(MemOpr, u8),
    /// `OR [mem8], reg8`
    OR_Mb_Rb(MemOpr, GprIndex8),
    /// `OR [mem32], imm32`
    OR_Md_Id(MemOpr, u32),
    /// `OR [mem32], reg32`
    OR_Md_Rd(MemOpr, GprIndex32),
    /// `OR [mem16], imm16`
    OR_Mw_Iw(MemOpr, u16),
    /// `OR [mem16], reg16`
    OR_Mw_Rw(MemOpr, GprIndex16),
    /// `OR reg8, imm8`
    OR_Rb_Ib(GprIndex8, u8),
    /// `OR reg8, [mem8]`
    OR_Rb_Mb(GprIndex8, MemOpr),
    /// `OR reg8, reg8`
    OR_Rb_Rb(GprIndex8, GprIndex8),
    /// `OR reg32, imm32`
    OR_Rd_Id(GprIndex32, u32),
    /// `OR reg32, [mem32]`
    OR_Rd_Md(GprIndex32, MemOpr),
    /// `OR reg32, reg32`
    OR_Rd_Rd(GprIndex32, GprIndex32),
    /// `OR reg16, imm16`
    OR_Rw_Iw(GprIndex16, u16),
    /// `OR reg16, [mem16]`
    OR_Rw_Mw(GprIndex16, MemOpr),
    /// `OR reg16, reg16`
    OR_Rw_Rw(GprIndex16, GprIndex16),
    /// `OUTSb (StringOpr)`
    OUTSB(StringOpr),
    /// `OUTSd (StringOpr)`
    OUTSD(StringOpr),
    /// `OUTSw (StringOpr)`
    OUTSW(StringOpr),
    /// `OUT dx, eax`
    OUT_Dx_Ad,
    /// `OUT dx, al`
    OUT_Dx_Al,
    /// `OUT dx, ax`
    OUT_Dx_Aw,
    /// `OUT imm8, eax`
    OUT_Ib_Ad(u8),
    /// `OUT imm8, al`
    OUT_Ib_Al(u8),
    /// `OUT imm8, ax`
    OUT_Ib_Aw(u8),
    /// `PAUSE`
    PAUSE,
    /// `POPA`
    POPA,
    /// `POPAD`
    POPAD,
    /// `POPCNT reg32, [mem32]`
    POPCNT_Rd_Md(GprIndex32, MemOpr),
    /// `POPCNT reg32, reg32`
    POPCNT_Rd_Rd(GprIndex32, GprIndex32),
    /// `POPCNT reg16, [mem16]`
    POPCNT_Rw_Mw(GprIndex16, MemOpr),
    /// `POPCNT reg16, reg16`
    POPCNT_Rw_Rw(GprIndex16, GprIndex16),
    /// `POPF`
    POPF,
    /// `POPFD`
    POPFD,
    /// `POP [mem32]`
    POP_Md(MemOpr),
    /// `POP [mem16]`
    POP_Mw(MemOpr),
    /// `POP reg32`
    POP_Rd(GprIndex32),
    /// `POP reg16`
    POP_Rw(GprIndex16),
    /// `o16: POP sr`
    POP_SrO16(SrIndex),
    /// `o32: POP sr`
    POP_SrO32(SrIndex),
    /// `PUSHA`
    PUSHA,
    /// `PUSHAD`
    PUSHAD,
    /// `PUSHF`
    PUSHF,
    /// `PUSHFD`
    PUSHFD,
    /// `PUSH imm32`
    PUSH_Id(u32),
    /// `PUSH imm16`
    PUSH_Iw(u16),
    /// `PUSH [mem32]`
    PUSH_Md(MemOpr),
    /// `PUSH [mem16]`
    PUSH_Mw(MemOpr),
    /// `PUSH reg32`
    PUSH_Rd(GprIndex32),
    /// `PUSH reg16`
    PUSH_Rw(GprIndex16),
    /// `o16: PUSH sr`
    PUSH_SrO16(SrIndex),
    /// `o32: PUSH sr`
    PUSH_SrO32(SrIndex),
    /// `RCL [mem8], cl`
    RCL_Mb_Cl(MemOpr),
    /// `RCL [mem8], imm8`
    RCL_Mb_Ib(MemOpr, u8),
    /// `RCL [mem32], cl`
    RCL_Md_Cl(MemOpr),
    /// `RCL [mem32], imm8`
    RCL_Md_Ib(MemOpr, u8),
    /// `RCL [mem16], cl`
    RCL_Mw_Cl(MemOpr),
    /// `RCL [mem16], imm8`
    RCL_Mw_Ib(MemOpr, u8),
    /// `RCL reg8, cl`
    RCL_Rb_Cl(GprIndex8),
    /// `RCL reg8, imm8`
    RCL_Rb_Ib(GprIndex8, u8),
    /// `RCL reg32, cl`
    RCL_Rd_Cl(GprIndex32),
    /// `RCL reg32, imm8`
    RCL_Rd_Ib(GprIndex32, u8),
    /// `RCL reg16, cl`
    RCL_Rw_Cl(GprIndex16),
    /// `RCL reg16, imm8`
    RCL_Rw_Ib(GprIndex16, u8),
    /// `RCR [mem8], cl`
    RCR_Mb_Cl(MemOpr),
    /// `RCR [mem8], imm8`
    RCR_Mb_Ib(MemOpr, u8),
    /// `RCR [mem32], cl`
    RCR_Md_Cl(MemOpr),
    /// `RCR [mem32], imm8`
    RCR_Md_Ib(MemOpr, u8),
    /// `RCR [mem16], cl`
    RCR_Mw_Cl(MemOpr),
    /// `RCR [mem16], imm8`
    RCR_Mw_Ib(MemOpr, u8),
    /// `RCR reg8, cl`
    RCR_Rb_Cl(GprIndex8),
    /// `RCR reg8, imm8`
    RCR_Rb_Ib(GprIndex8, u8),
    /// `RCR reg32, cl`
    RCR_Rd_Cl(GprIndex32),
    /// `RCR reg32, imm8`
    RCR_Rd_Ib(GprIndex32, u8),
    /// `RCR reg16, cl`
    RCR_Rw_Cl(GprIndex16),
    /// `RCR reg16, imm8`
    RCR_Rw_Ib(GprIndex16, u8),
    /// `RDMSR`
    RDMSR,
    /// `RDTSC`
    RDTSC,
    /// `RDTSCP`
    RDTSCP,
    /// `REPNZ CMPSb (StringOpr)`
    REPNZ_CMPSB(StringOpr),
    /// `REPNZ CMPSd (StringOpr)`
    REPNZ_CMPSD(StringOpr),
    /// `REPNZ CMPSw (StringOpr)`
    REPNZ_CMPSW(StringOpr),
    /// `REPNZ SCASb (StringOprEs)`
    REPNZ_SCASB(StringOprEs),
    /// `REPNZ SCASd (StringOprEs)`
    REPNZ_SCASD(StringOprEs),
    /// `REPNZ SCASw (StringOprEs)`
    REPNZ_SCASW(StringOprEs),
    /// `REPZ CMPSb (StringOpr)`
    REPZ_CMPSB(StringOpr),
    /// `REPZ CMPSd (StringOpr)`
    REPZ_CMPSD(StringOpr),
    /// `REPZ CMPSw (StringOpr)`
    REPZ_CMPSW(StringOpr),
    /// `REPZ SCASb (StringOprEs)`
    REPZ_SCASB(StringOprEs),
    /// `REPZ SCASd (StringOprEs)`
    REPZ_SCASD(StringOprEs),
    /// `REPZ SCASw (StringOprEs)`
    REPZ_SCASW(StringOprEs),
    /// `REP INSb (StringOprEs)`
    REP_INSB(StringOprEs),
    /// `REP INSd (StringOprEs)`
    REP_INSD(StringOprEs),
    /// `REP INSw (StringOprEs)`
    REP_INSW(StringOprEs),
    /// `REP LODSb (StringOpr)`
    REP_LODSB(StringOpr),
    /// `REP LODSd (StringOpr)`
    REP_LODSD(StringOpr),
    /// `REP LODSw (StringOpr)`
    REP_LODSW(StringOpr),
    /// `REP MOVSb (StringOpr)`
    REP_MOVSB(StringOpr),
    /// `REP MOVSd (StringOpr)`
    REP_MOVSD(StringOpr),
    /// `REP MOVSw (StringOpr)`
    REP_MOVSW(StringOpr),
    /// `REP OUTSb (StringOpr)`
    REP_OUTSB(StringOpr),
    /// `REP OUTSd (StringOpr)`
    REP_OUTSD(StringOpr),
    /// `REP OUTSw (StringOpr)`
    REP_OUTSW(StringOpr),
    /// `REP STOSb (StringOprEs)`
    REP_STOSB(StringOprEs),
    /// `REP STOSd (StringOprEs)`
    REP_STOSD(StringOprEs),
    /// `REP STOSw (StringOprEs)`
    REP_STOSW(StringOprEs),
    /// `d16: RETF`
    RETF_D16(u16),
    /// `d32: RETF`
    RETF_D32(u16),
    /// `d16: RET`
    RET_D16(u16),
    /// `d32: RET`
    RET_D32(u16),
    /// `ROL [mem8], cl`
    ROL_Mb_Cl(MemOpr),
    /// `ROL [mem8], imm8`
    ROL_Mb_Ib(MemOpr, u8),
    /// `ROL [mem32], cl`
    ROL_Md_Cl(MemOpr),
    /// `ROL [mem32], imm8`
    ROL_Md_Ib(MemOpr, u8),
    /// `ROL [mem16], cl`
    ROL_Mw_Cl(MemOpr),
    /// `ROL [mem16], imm8`
    ROL_Mw_Ib(MemOpr, u8),
    /// `ROL reg8, cl`
    ROL_Rb_Cl(GprIndex8),
    /// `ROL reg8, imm8`
    ROL_Rb_Ib(GprIndex8, u8),
    /// `ROL reg32, cl`
    ROL_Rd_Cl(GprIndex32),
    /// `ROL reg32, imm8`
    ROL_Rd_Ib(GprIndex32, u8),
    /// `ROL reg16, cl`
    ROL_Rw_Cl(GprIndex16),
    /// `ROL reg16, imm8`
    ROL_Rw_Ib(GprIndex16, u8),
    /// `ROR [mem8], cl`
    ROR_Mb_Cl(MemOpr),
    /// `ROR [mem8], imm8`
    ROR_Mb_Ib(MemOpr, u8),
    /// `ROR [mem32], cl`
    ROR_Md_Cl(MemOpr),
    /// `ROR [mem32], imm8`
    ROR_Md_Ib(MemOpr, u8),
    /// `ROR [mem16], cl`
    ROR_Mw_Cl(MemOpr),
    /// `ROR [mem16], imm8`
    ROR_Mw_Ib(MemOpr, u8),
    /// `ROR reg8, cl`
    ROR_Rb_Cl(GprIndex8),
    /// `ROR reg8, imm8`
    ROR_Rb_Ib(GprIndex8, u8),
    /// `ROR reg32, cl`
    ROR_Rd_Cl(GprIndex32),
    /// `ROR reg32, imm8`
    ROR_Rd_Ib(GprIndex32, u8),
    /// `ROR reg16, cl`
    ROR_Rw_Cl(GprIndex16),
    /// `ROR reg16, imm8`
    ROR_Rw_Ib(GprIndex16, u8),
    /// `RSM`
    RSM,
    /// `SAHF`
    SAHF,
    /// `SALC`
    SALC,
    /// `SAR [mem8], cl`
    SAR_Mb_Cl(MemOpr),
    /// `SAR [mem8], imm8`
    SAR_Mb_Ib(MemOpr, u8),
    /// `SAR [mem32], cl`
    SAR_Md_Cl(MemOpr),
    /// `SAR [mem32], imm8`
    SAR_Md_Ib(MemOpr, u8),
    /// `SAR [mem16], cl`
    SAR_Mw_Cl(MemOpr),
    /// `SAR [mem16], imm8`
    SAR_Mw_Ib(MemOpr, u8),
    /// `SAR reg8, cl`
    SAR_Rb_Cl(GprIndex8),
    /// `SAR reg8, imm8`
    SAR_Rb_Ib(GprIndex8, u8),
    /// `SAR reg32, cl`
    SAR_Rd_Cl(GprIndex32),
    /// `SAR reg32, imm8`
    SAR_Rd_Ib(GprIndex32, u8),
    /// `SAR reg16, cl`
    SAR_Rw_Cl(GprIndex16),
    /// `SAR reg16, imm8`
    SAR_Rw_Ib(GprIndex16, u8),
    /// `SBB [mem8], imm8`
    SBB_Mb_Ib(MemOpr, u8),
    /// `SBB [mem8], reg8`
    SBB_Mb_Rb(MemOpr, GprIndex8),
    /// `SBB [mem32], imm32`
    SBB_Md_Id(MemOpr, u32),
    /// `SBB [mem32], reg32`
    SBB_Md_Rd(MemOpr, GprIndex32),
    /// `SBB [mem16], imm16`
    SBB_Mw_Iw(MemOpr, u16),
    /// `SBB [mem16], reg16`
    SBB_Mw_Rw(MemOpr, GprIndex16),
    /// `SBB reg8, imm8`
    SBB_Rb_Ib(GprIndex8, u8),
    /// `SBB reg8, [mem8]`
    SBB_Rb_Mb(GprIndex8, MemOpr),
    /// `SBB reg8, reg8`
    SBB_Rb_Rb(GprIndex8, GprIndex8),
    /// `SBB reg32, imm32`
    SBB_Rd_Id(GprIndex32, u32),
    /// `SBB reg32, [mem32]`
    SBB_Rd_Md(GprIndex32, MemOpr),
    /// `SBB reg32, reg32`
    SBB_Rd_Rd(GprIndex32, GprIndex32),
    /// `SBB reg16, imm16`
    SBB_Rw_Iw(GprIndex16, u16),
    /// `SBB reg16, [mem16]`
    SBB_Rw_Mw(GprIndex16, MemOpr),
    /// `SBB reg16, reg16`
    SBB_Rw_Rw(GprIndex16, GprIndex16),
    /// `SCASb (StringOprEs)`
    SCASB(StringOprEs),
    /// `SCASd (StringOprEs)`
    SCASD(StringOprEs),
    /// `SCASw (StringOprEs)`
    SCASW(StringOprEs),
    /// `SETCC cc, [mem8]`
    SETCC_Mb(CC, MemOpr),
    /// `SETCC cc, reg8`
    SETCC_Rb(CC, GprIndex8),
    /// `SGDT [mem]`
    SGDT_Mp(MemOpr),
    /// `SHLD [mem32], reg32, cl`
    SHLD_Md_Rd_Cl(MemOpr, GprIndex32),
    /// `SHLD [mem32], reg32, imm8`
    SHLD_Md_Rd_Ib(MemOpr, GprIndex32, u8),
    /// `SHLD [mem16], reg16, cl`
    SHLD_Mw_Rw_Cl(MemOpr, GprIndex16),
    /// `SHLD [mem16], reg16, imm8`
    SHLD_Mw_Rw_Ib(MemOpr, GprIndex16, u8),
    /// `SHLD reg32, reg32, cl`
    SHLD_Rd_Rd_Cl(GprIndex32, GprIndex32),
    /// `SHLD reg32, reg32, imm8`
    SHLD_Rd_Rd_Ib(GprIndex32, GprIndex32, u8),
    /// `SHLD reg16, reg16, cl`
    SHLD_Rw_Rw_Cl(GprIndex16, GprIndex16),
    /// `SHLD reg16, reg16, imm8`
    SHLD_Rw_Rw_Ib(GprIndex16, GprIndex16, u8),
    /// `SHL [mem8], cl`
    SHL_Mb_Cl(MemOpr),
    /// `SHL [mem8], imm8`
    SHL_Mb_Ib(MemOpr, u8),
    /// `SHL [mem32], cl`
    SHL_Md_Cl(MemOpr),
    /// `SHL [mem32], imm8`
    SHL_Md_Ib(MemOpr, u8),
    /// `SHL [mem16], cl`
    SHL_Mw_Cl(MemOpr),
    /// `SHL [mem16], imm8`
    SHL_Mw_Ib(MemOpr, u8),
    /// `SHL reg8, cl`
    SHL_Rb_Cl(GprIndex8),
    /// `SHL reg8, imm8`
    SHL_Rb_Ib(GprIndex8, u8),
    /// `SHL reg32, cl`
    SHL_Rd_Cl(GprIndex32),
    /// `SHL reg32, imm8`
    SHL_Rd_Ib(GprIndex32, u8),
    /// `SHL reg16, cl`
    SHL_Rw_Cl(GprIndex16),
    /// `SHL reg16, imm8`
    SHL_Rw_Ib(GprIndex16, u8),
    /// `SHRD [mem32], reg32, cl`
    SHRD_Md_Rd_Cl(MemOpr, GprIndex32),
    /// `SHRD [mem32], reg32, imm8`
    SHRD_Md_Rd_Ib(MemOpr, GprIndex32, u8),
    /// `SHRD [mem16], reg16, cl`
    SHRD_Mw_Rw_Cl(MemOpr, GprIndex16),
    /// `SHRD [mem16], reg16, imm8`
    SHRD_Mw_Rw_Ib(MemOpr, GprIndex16, u8),
    /// `SHRD reg32, reg32, cl`
    SHRD_Rd_Rd_Cl(GprIndex32, GprIndex32),
    /// `SHRD reg32, reg32, imm8`
    SHRD_Rd_Rd_Ib(GprIndex32, GprIndex32, u8),
    /// `SHRD reg16, reg16, cl`
    SHRD_Rw_Rw_Cl(GprIndex16, GprIndex16),
    /// `SHRD reg16, reg16, imm8`
    SHRD_Rw_Rw_Ib(GprIndex16, GprIndex16, u8),
    /// `SHR [mem8], cl`
    SHR_Mb_Cl(MemOpr),
    /// `SHR [mem8], imm8`
    SHR_Mb_Ib(MemOpr, u8),
    /// `SHR [mem32], cl`
    SHR_Md_Cl(MemOpr),
    /// `SHR [mem32], imm8`
    SHR_Md_Ib(MemOpr, u8),
    /// `SHR [mem16], cl`
    SHR_Mw_Cl(MemOpr),
    /// `SHR [mem16], imm8`
    SHR_Mw_Ib(MemOpr, u8),
    /// `SHR reg8, cl`
    SHR_Rb_Cl(GprIndex8),
    /// `SHR reg8, imm8`
    SHR_Rb_Ib(GprIndex8, u8),
    /// `SHR reg32, cl`
    SHR_Rd_Cl(GprIndex32),
    /// `SHR reg32, imm8`
    SHR_Rd_Ib(GprIndex32, u8),
    /// `SHR reg16, cl`
    SHR_Rw_Cl(GprIndex16),
    /// `SHR reg16, imm8`
    SHR_Rw_Ib(GprIndex16, u8),
    /// `SIDT [mem]`
    SIDT_Mp(MemOpr),
    /// `simd op`
    SIMD(SimdOp),
    /// `simd op reg, [mem], imm8`
    SIMD_M(SimdOp, SimdReg, MemOpr, u8),
    /// `simd op reg, r/m-reg, imm8`
    SIMD_R(SimdOp, SimdReg, SimdReg, u8),
    /// `simd op reg, r/m-reg, implicit [seg:(e)di]`
    SIMD_S(SimdOp, SimdReg, SimdReg, StringOpr),
    /// `SLDT [mem32]`
    SLDT_Md(MemOpr),
    /// `SLDT [mem16]`
    SLDT_Mw(MemOpr),
    /// `SLDT reg32`
    SLDT_Rd(GprIndex32),
    /// `SLDT reg16`
    SLDT_Rw(GprIndex16),
    /// `SMSW [mem16]`
    SMSW_Mw(MemOpr),
    /// `SMSW reg16`
    SMSW_Rw(GprIndex16),
    /// `STC`
    STC,
    /// `STD`
    STD,
    /// `STI`
    STI,
    /// `STOSb (StringOprEs)`
    STOSB(StringOprEs),
    /// `STOSd (StringOprEs)`
    STOSD(StringOprEs),
    /// `STOSw (StringOprEs)`
    STOSW(StringOprEs),
    /// `STR [mem32]`
    STR_Md(MemOpr),
    /// `STR [mem16]`
    STR_Mw(MemOpr),
    /// `STR reg32`
    STR_Rd(GprIndex32),
    /// `STR reg16`
    STR_Rw(GprIndex16),
    /// `SUB [mem8], imm8`
    SUB_Mb_Ib(MemOpr, u8),
    /// `SUB [mem8], reg8`
    SUB_Mb_Rb(MemOpr, GprIndex8),
    /// `SUB [mem32], imm32`
    SUB_Md_Id(MemOpr, u32),
    /// `SUB [mem32], reg32`
    SUB_Md_Rd(MemOpr, GprIndex32),
    /// `SUB [mem16], imm16`
    SUB_Mw_Iw(MemOpr, u16),
    /// `SUB [mem16], reg16`
    SUB_Mw_Rw(MemOpr, GprIndex16),
    /// `SUB reg8, imm8`
    SUB_Rb_Ib(GprIndex8, u8),
    /// `SUB reg8, [mem8]`
    SUB_Rb_Mb(GprIndex8, MemOpr),
    /// `SUB reg8, reg8`
    SUB_Rb_Rb(GprIndex8, GprIndex8),
    /// `SUB reg32, imm32`
    SUB_Rd_Id(GprIndex32, u32),
    /// `SUB reg32, [mem32]`
    SUB_Rd_Md(GprIndex32, MemOpr),
    /// `SUB reg32, reg32`
    SUB_Rd_Rd(GprIndex32, GprIndex32),
    /// `SUB reg16, imm16`
    SUB_Rw_Iw(GprIndex16, u16),
    /// `SUB reg16, [mem16]`
    SUB_Rw_Mw(GprIndex16, MemOpr),
    /// `SUB reg16, reg16`
    SUB_Rw_Rw(GprIndex16, GprIndex16),
    /// `SWAPGS`
    SWAPGS,
    /// `SYSCALL`
    SYSCALL,
    /// `SYSENTER`
    SYSENTER,
    /// `SYSEXIT`
    SYSEXIT,
    /// `SYSRET`
    SYSRET,
    /// `TEST [mem8], imm8`
    TEST_Mb_Ib(MemOpr, u8),
    /// `TEST [mem8], reg8`
    TEST_Mb_Rb(MemOpr, GprIndex8),
    /// `TEST [mem32], imm32`
    TEST_Md_Id(MemOpr, u32),
    /// `TEST [mem32], reg32`
    TEST_Md_Rd(MemOpr, GprIndex32),
    /// `TEST [mem16], imm16`
    TEST_Mw_Iw(MemOpr, u16),
    /// `TEST [mem16], reg16`
    TEST_Mw_Rw(MemOpr, GprIndex16),
    /// `TEST reg8, imm8`
    TEST_Rb_Ib(GprIndex8, u8),
    /// `TEST reg8, reg8`
    TEST_Rb_Rb(GprIndex8, GprIndex8),
    /// `TEST reg32, imm32`
    TEST_Rd_Id(GprIndex32, u32),
    /// `TEST reg32, reg32`
    TEST_Rd_Rd(GprIndex32, GprIndex32),
    /// `TEST reg16, imm16`
    TEST_Rw_Iw(GprIndex16, u16),
    /// `TEST reg16, reg16`
    TEST_Rw_Rw(GprIndex16, GprIndex16),
    /// `UD`
    UD,
    /// `VERR [mem16]`
    VERR_Mw(MemOpr),
    /// `VERR reg16`
    VERR_Rw(GprIndex16),
    /// `VERW [mem16]`
    VERW_Mw(MemOpr),
    /// `VERW reg16`
    VERW_Rw(GprIndex16),
    /// `WAIT`
    WAIT,
    /// `WBINVD`
    WBINVD,
    /// `WRMSR`
    WRMSR,
    /// `XADD [mem8], reg8`
    XADD_Mb_Rb(MemOpr, GprIndex8),
    /// `XADD [mem32], reg32`
    XADD_Md_Rd(MemOpr, GprIndex32),
    /// `XADD [mem16], reg16`
    XADD_Mw_Rw(MemOpr, GprIndex16),
    /// `XADD reg8, reg8`
    XADD_Rb_Rb(GprIndex8, GprIndex8),
    /// `XADD reg32, reg32`
    XADD_Rd_Rd(GprIndex32, GprIndex32),
    /// `XADD reg16, reg16`
    XADD_Rw_Rw(GprIndex16, GprIndex16),
    /// `XCHG [mem8], reg8`
    XCHG_Mb_Rb(MemOpr, GprIndex8),
    /// `XCHG [mem32], reg32`
    XCHG_Md_Rd(MemOpr, GprIndex32),
    /// `XCHG [mem16], reg16`
    XCHG_Mw_Rw(MemOpr, GprIndex16),
    /// `XCHG reg8, reg8`
    XCHG_Rb_Rb(GprIndex8, GprIndex8),
    /// `XCHG reg32, reg32`
    XCHG_Rd_Rd(GprIndex32, GprIndex32),
    /// `XCHG reg16, reg16`
    XCHG_Rw_Rw(GprIndex16, GprIndex16),
    /// `XGETBV`
    XGETBV,
    /// `XLAT [mem8]`
    XLAT_Mb(MemOpr),
    /// `XOR [mem8], imm8`
    XOR_Mb_Ib(MemOpr, u8),
    /// `XOR [mem8], reg8`
    XOR_Mb_Rb(MemOpr, GprIndex8),
    /// `XOR [mem32], imm32`
    XOR_Md_Id(MemOpr, u32),
    /// `XOR [mem32], reg32`
    XOR_Md_Rd(MemOpr, GprIndex32),
    /// `XOR [mem16], imm16`
    XOR_Mw_Iw(MemOpr, u16),
    /// `XOR [mem16], reg16`
    XOR_Mw_Rw(MemOpr, GprIndex16),
    /// `XOR reg8, imm8`
    XOR_Rb_Ib(GprIndex8, u8),
    /// `XOR reg8, [mem8]`
    XOR_Rb_Mb(GprIndex8, MemOpr),
    /// `XOR reg8, reg8`
    XOR_Rb_Rb(GprIndex8, GprIndex8),
    /// `XOR reg32, imm32`
    XOR_Rd_Id(GprIndex32, u32),
    /// `XOR reg32, [mem32]`
    XOR_Rd_Md(GprIndex32, MemOpr),
    /// `XOR reg32, reg32`
    XOR_Rd_Rd(GprIndex32, GprIndex32),
    /// `XOR reg16, imm16`
    XOR_Rw_Iw(GprIndex16, u16),
    /// `XOR reg16, [mem16]`
    XOR_Rw_Mw(GprIndex16, MemOpr),
    /// `XOR reg16, reg16`
    XOR_Rw_Rw(GprIndex16, GprIndex16),
    /// `XSETBV`
    XSETBV,
}

impl IrOp {
    pub fn mnemonic(&self) -> Mnemonic {
        match self {
            Self::AAA => Mnemonic::AAA,
            Self::AAD_Ib(_) => Mnemonic::AAD,
            Self::AAM_Ib(_) => Mnemonic::AAM,
            Self::AAS => Mnemonic::AAS,
            Self::ADC_Mb_Ib(_, _) => Mnemonic::ADC,
            Self::ADC_Mb_Rb(_, _) => Mnemonic::ADC,
            Self::ADC_Md_Id(_, _) => Mnemonic::ADC,
            Self::ADC_Md_Rd(_, _) => Mnemonic::ADC,
            Self::ADC_Mw_Iw(_, _) => Mnemonic::ADC,
            Self::ADC_Mw_Rw(_, _) => Mnemonic::ADC,
            Self::ADC_Rb_Ib(_, _) => Mnemonic::ADC,
            Self::ADC_Rb_Mb(_, _) => Mnemonic::ADC,
            Self::ADC_Rb_Rb(_, _) => Mnemonic::ADC,
            Self::ADC_Rd_Id(_, _) => Mnemonic::ADC,
            Self::ADC_Rd_Md(_, _) => Mnemonic::ADC,
            Self::ADC_Rd_Rd(_, _) => Mnemonic::ADC,
            Self::ADC_Rw_Iw(_, _) => Mnemonic::ADC,
            Self::ADC_Rw_Mw(_, _) => Mnemonic::ADC,
            Self::ADC_Rw_Rw(_, _) => Mnemonic::ADC,
            Self::ADD_Mb_Ib(_, _) => Mnemonic::ADD,
            Self::ADD_Mb_Rb(_, _) => Mnemonic::ADD,
            Self::ADD_Md_Id(_, _) => Mnemonic::ADD,
            Self::ADD_Md_Rd(_, _) => Mnemonic::ADD,
            Self::ADD_Mw_Iw(_, _) => Mnemonic::ADD,
            Self::ADD_Mw_Rw(_, _) => Mnemonic::ADD,
            Self::ADD_Rb_Ib(_, _) => Mnemonic::ADD,
            Self::ADD_Rb_Mb(_, _) => Mnemonic::ADD,
            Self::ADD_Rb_Rb(_, _) => Mnemonic::ADD,
            Self::ADD_Rd_Id(_, _) => Mnemonic::ADD,
            Self::ADD_Rd_Md(_, _) => Mnemonic::ADD,
            Self::ADD_Rd_Rd(_, _) => Mnemonic::ADD,
            Self::ADD_Rw_Iw(_, _) => Mnemonic::ADD,
            Self::ADD_Rw_Mw(_, _) => Mnemonic::ADD,
            Self::ADD_Rw_Rw(_, _) => Mnemonic::ADD,
            Self::AND_Mb_Ib(_, _) => Mnemonic::AND,
            Self::AND_Mb_Rb(_, _) => Mnemonic::AND,
            Self::AND_Md_Id(_, _) => Mnemonic::AND,
            Self::AND_Md_Rd(_, _) => Mnemonic::AND,
            Self::AND_Mw_Iw(_, _) => Mnemonic::AND,
            Self::AND_Mw_Rw(_, _) => Mnemonic::AND,
            Self::AND_Rb_Ib(_, _) => Mnemonic::AND,
            Self::AND_Rb_Mb(_, _) => Mnemonic::AND,
            Self::AND_Rb_Rb(_, _) => Mnemonic::AND,
            Self::AND_Rd_Id(_, _) => Mnemonic::AND,
            Self::AND_Rd_Md(_, _) => Mnemonic::AND,
            Self::AND_Rd_Rd(_, _) => Mnemonic::AND,
            Self::AND_Rw_Iw(_, _) => Mnemonic::AND,
            Self::AND_Rw_Mw(_, _) => Mnemonic::AND,
            Self::AND_Rw_Rw(_, _) => Mnemonic::AND,
            Self::ARPL_Mw_Rw(_, _) => Mnemonic::ARPL,
            Self::ARPL_Rw_Rw(_, _) => Mnemonic::ARPL,
            Self::BOUND_Rd_Md(_, _) => Mnemonic::BOUND,
            Self::BOUND_Rw_Mw(_, _) => Mnemonic::BOUND,
            Self::BSF_Rd_Md(_, _) => Mnemonic::BSF,
            Self::BSF_Rd_Rd(_, _) => Mnemonic::BSF,
            Self::BSF_Rw_Mw(_, _) => Mnemonic::BSF,
            Self::BSF_Rw_Rw(_, _) => Mnemonic::BSF,
            Self::BSR_Rd_Md(_, _) => Mnemonic::BSR,
            Self::BSR_Rd_Rd(_, _) => Mnemonic::BSR,
            Self::BSR_Rw_Mw(_, _) => Mnemonic::BSR,
            Self::BSR_Rw_Rw(_, _) => Mnemonic::BSR,
            Self::BSWAP_Rd(_) => Mnemonic::BSWAP,
            Self::BTC_Md_Ib(_, _) => Mnemonic::BTC,
            Self::BTC_Md_Rd(_, _) => Mnemonic::BTC,
            Self::BTC_Mw_Ib(_, _) => Mnemonic::BTC,
            Self::BTC_Mw_Rw(_, _) => Mnemonic::BTC,
            Self::BTC_Rd_Ib(_, _) => Mnemonic::BTC,
            Self::BTC_Rd_Rd(_, _) => Mnemonic::BTC,
            Self::BTC_Rw_Ib(_, _) => Mnemonic::BTC,
            Self::BTC_Rw_Rw(_, _) => Mnemonic::BTC,
            Self::BTR_Md_Ib(_, _) => Mnemonic::BTR,
            Self::BTR_Md_Rd(_, _) => Mnemonic::BTR,
            Self::BTR_Mw_Ib(_, _) => Mnemonic::BTR,
            Self::BTR_Mw_Rw(_, _) => Mnemonic::BTR,
            Self::BTR_Rd_Ib(_, _) => Mnemonic::BTR,
            Self::BTR_Rd_Rd(_, _) => Mnemonic::BTR,
            Self::BTR_Rw_Ib(_, _) => Mnemonic::BTR,
            Self::BTR_Rw_Rw(_, _) => Mnemonic::BTR,
            Self::BTS_Md_Ib(_, _) => Mnemonic::BTS,
            Self::BTS_Md_Rd(_, _) => Mnemonic::BTS,
            Self::BTS_Mw_Ib(_, _) => Mnemonic::BTS,
            Self::BTS_Mw_Rw(_, _) => Mnemonic::BTS,
            Self::BTS_Rd_Ib(_, _) => Mnemonic::BTS,
            Self::BTS_Rd_Rd(_, _) => Mnemonic::BTS,
            Self::BTS_Rw_Ib(_, _) => Mnemonic::BTS,
            Self::BTS_Rw_Rw(_, _) => Mnemonic::BTS,
            Self::BT_Md_Ib(_, _) => Mnemonic::BT,
            Self::BT_Md_Rd(_, _) => Mnemonic::BT,
            Self::BT_Mw_Ib(_, _) => Mnemonic::BT,
            Self::BT_Mw_Rw(_, _) => Mnemonic::BT,
            Self::BT_Rd_Ib(_, _) => Mnemonic::BT,
            Self::BT_Rd_Rd(_, _) => Mnemonic::BT,
            Self::BT_Rw_Ib(_, _) => Mnemonic::BT,
            Self::BT_Rw_Rw(_, _) => Mnemonic::BT,
            Self::CALLF_ApO16(_, _) => Mnemonic::CALLF,
            Self::CALLF_ApO32(_, _) => Mnemonic::CALLF,
            Self::CALLF_MpO16(_) => Mnemonic::CALLF,
            Self::CALLF_MpO32(_) => Mnemonic::CALLF,
            Self::CALL_Jd(_) => Mnemonic::CALL,
            Self::CALL_Jw(_) => Mnemonic::CALL,
            Self::CALL_Md(_) => Mnemonic::CALL,
            Self::CALL_Mw(_) => Mnemonic::CALL,
            Self::CALL_Rd(_) => Mnemonic::CALL,
            Self::CALL_Rw(_) => Mnemonic::CALL,
            Self::CDQ => Mnemonic::CDQ,
            Self::CLC => Mnemonic::CLC,
            Self::CLD => Mnemonic::CLD,
            Self::CLI => Mnemonic::CLI,
            Self::CLTS => Mnemonic::CLTS,
            Self::CMC => Mnemonic::CMC,
            Self::CMOV_Rd_Md(_, _, _) => Mnemonic::CMOV,
            Self::CMOV_Rd_Rd(_, _, _) => Mnemonic::CMOV,
            Self::CMOV_Rw_Mw(_, _, _) => Mnemonic::CMOV,
            Self::CMOV_Rw_Rw(_, _, _) => Mnemonic::CMOV,
            Self::CMPSB(_) => Mnemonic::CMPS,
            Self::CMPSD(_) => Mnemonic::CMPS,
            Self::CMPSW(_) => Mnemonic::CMPS,
            Self::CMPXCHG_Mb_Rb(_, _) => Mnemonic::CMPXCHG,
            Self::CMPXCHG_Md_Rd(_, _) => Mnemonic::CMPXCHG,
            Self::CMPXCHG_Mw_Rw(_, _) => Mnemonic::CMPXCHG,
            Self::CMPXCHG_Rb_Rb(_, _) => Mnemonic::CMPXCHG,
            Self::CMPXCHG_Rd_Rd(_, _) => Mnemonic::CMPXCHG,
            Self::CMPXCHG_Rw_Rw(_, _) => Mnemonic::CMPXCHG,
            Self::CMP_Mb_Ib(_, _) => Mnemonic::CMP,
            Self::CMP_Mb_Rb(_, _) => Mnemonic::CMP,
            Self::CMP_Md_Id(_, _) => Mnemonic::CMP,
            Self::CMP_Md_Rd(_, _) => Mnemonic::CMP,
            Self::CMP_Mw_Iw(_, _) => Mnemonic::CMP,
            Self::CMP_Mw_Rw(_, _) => Mnemonic::CMP,
            Self::CMP_Rb_Ib(_, _) => Mnemonic::CMP,
            Self::CMP_Rb_Mb(_, _) => Mnemonic::CMP,
            Self::CMP_Rb_Rb(_, _) => Mnemonic::CMP,
            Self::CMP_Rd_Id(_, _) => Mnemonic::CMP,
            Self::CMP_Rd_Md(_, _) => Mnemonic::CMP,
            Self::CMP_Rd_Rd(_, _) => Mnemonic::CMP,
            Self::CMP_Rw_Iw(_, _) => Mnemonic::CMP,
            Self::CMP_Rw_Mw(_, _) => Mnemonic::CMP,
            Self::CMP_Rw_Rw(_, _) => Mnemonic::CMP,
            Self::CPUID => Mnemonic::CPUID,
            Self::CWD => Mnemonic::CWD,
            Self::DAA => Mnemonic::DAA,
            Self::DAS => Mnemonic::DAS,
            Self::DEC_Mb(_) => Mnemonic::DEC,
            Self::DEC_Md(_) => Mnemonic::DEC,
            Self::DEC_Mw(_) => Mnemonic::DEC,
            Self::DEC_Rb(_) => Mnemonic::DEC,
            Self::DEC_Rd(_) => Mnemonic::DEC,
            Self::DEC_Rw(_) => Mnemonic::DEC,
            Self::DIV_Mb(_) => Mnemonic::DIV,
            Self::DIV_Md(_) => Mnemonic::DIV,
            Self::DIV_Mw(_) => Mnemonic::DIV,
            Self::DIV_Rb(_) => Mnemonic::DIV,
            Self::DIV_Rd(_) => Mnemonic::DIV,
            Self::DIV_Rw(_) => Mnemonic::DIV,
            Self::ENTER_Iw_Ib(_, _) => Mnemonic::ENTER,
            Self::ESC_Mb(_, _, _) => Mnemonic::ESC,
            Self::ESC_Rb(_, _, _) => Mnemonic::ESC,
            Self::HLT => Mnemonic::HLT,
            Self::ICEBP => Mnemonic::ICEBP,
            Self::IDIV_Mb(_) => Mnemonic::IDIV,
            Self::IDIV_Md(_) => Mnemonic::IDIV,
            Self::IDIV_Mw(_) => Mnemonic::IDIV,
            Self::IDIV_Rb(_) => Mnemonic::IDIV,
            Self::IDIV_Rd(_) => Mnemonic::IDIV,
            Self::IDIV_Rw(_) => Mnemonic::IDIV,
            Self::IMUL_Mb(_) => Mnemonic::IMUL,
            Self::IMUL_Md(_) => Mnemonic::IMUL,
            Self::IMUL_Mw(_) => Mnemonic::IMUL,
            Self::IMUL_Rb(_) => Mnemonic::IMUL,
            Self::IMUL_Rd(_) => Mnemonic::IMUL,
            Self::IMUL_Rd_Md(_, _) => Mnemonic::IMUL,
            Self::IMUL_Rd_Md_Id(_, _, _) => Mnemonic::IMUL,
            Self::IMUL_Rd_Rd(_, _) => Mnemonic::IMUL,
            Self::IMUL_Rd_Rd_Id(_, _, _) => Mnemonic::IMUL,
            Self::IMUL_Rw(_) => Mnemonic::IMUL,
            Self::IMUL_Rw_Mw(_, _) => Mnemonic::IMUL,
            Self::IMUL_Rw_Mw_Iw(_, _, _) => Mnemonic::IMUL,
            Self::IMUL_Rw_Rw(_, _) => Mnemonic::IMUL,
            Self::IMUL_Rw_Rw_Iw(_, _, _) => Mnemonic::IMUL,
            Self::INC_Mb(_) => Mnemonic::INC,
            Self::INC_Md(_) => Mnemonic::INC,
            Self::INC_Mw(_) => Mnemonic::INC,
            Self::INC_Rb(_) => Mnemonic::INC,
            Self::INC_Rd(_) => Mnemonic::INC,
            Self::INC_Rw(_) => Mnemonic::INC,
            Self::INSB(_) => Mnemonic::INS,
            Self::INSD(_) => Mnemonic::INS,
            Self::INSW(_) => Mnemonic::INS,
            Self::INT3 => Mnemonic::INT3,
            Self::INTO => Mnemonic::INTO,
            Self::INT_Ib(_) => Mnemonic::INT,
            Self::INVD => Mnemonic::INVD,
            Self::INVLPG_Mb(_) => Mnemonic::INVLPG,
            Self::INVLPG_Md(_) => Mnemonic::INVLPG,
            Self::INVLPG_Mw(_) => Mnemonic::INVLPG,
            Self::INVLPG_Rb(_) => Mnemonic::INVLPG,
            Self::INVLPG_Rd(_) => Mnemonic::INVLPG,
            Self::INVLPG_Rw(_) => Mnemonic::INVLPG,
            Self::IN_Ad_Dx => Mnemonic::IN,
            Self::IN_Ad_Ib(_) => Mnemonic::IN,
            Self::IN_Al_Dx => Mnemonic::IN,
            Self::IN_Al_Ib(_) => Mnemonic::IN,
            Self::IN_Aw_Dx => Mnemonic::IN,
            Self::IN_Aw_Ib(_) => Mnemonic::IN,
            Self::IRET => Mnemonic::IRET,
            Self::IRETD => Mnemonic::IRETD,
            Self::JCC_Jd(_, _) => Mnemonic::JCC,
            Self::JCC_Jw(_, _) => Mnemonic::JCC,
            Self::JCXZ_Jd(_) => Mnemonic::JCXZ,
            Self::JCXZ_Jw(_) => Mnemonic::JCXZ,
            Self::JECXZ_Jd(_) => Mnemonic::JECXZ,
            Self::JECXZ_Jw(_) => Mnemonic::JECXZ,
            Self::JMPF_ApO16(_, _) => Mnemonic::JMPF,
            Self::JMPF_ApO32(_, _) => Mnemonic::JMPF,
            Self::JMPF_MpO16(_) => Mnemonic::JMPF,
            Self::JMPF_MpO32(_) => Mnemonic::JMPF,
            Self::JMP_Jd(_) => Mnemonic::JMP,
            Self::JMP_Jw(_) => Mnemonic::JMP,
            Self::JMP_Md(_) => Mnemonic::JMP,
            Self::JMP_Mw(_) => Mnemonic::JMP,
            Self::JMP_Rd(_) => Mnemonic::JMP,
            Self::JMP_Rw(_) => Mnemonic::JMP,
            Self::LAHF => Mnemonic::LAHF,
            Self::LAR_Rd_Mw(_, _) => Mnemonic::LAR,
            Self::LAR_Rd_Rw(_, _) => Mnemonic::LAR,
            Self::LAR_Rw_Mw(_, _) => Mnemonic::LAR,
            Self::LAR_Rw_Rw(_, _) => Mnemonic::LAR,
            Self::LDS_Rd_Md(_, _) => Mnemonic::LDS,
            Self::LDS_Rw_Mw(_, _) => Mnemonic::LDS,
            Self::LEAVE => Mnemonic::LEAVE,
            Self::LEA_Rd_Md(_, _) => Mnemonic::LEA,
            Self::LEA_Rw_Mw(_, _) => Mnemonic::LEA,
            Self::LES_Rd_Md(_, _) => Mnemonic::LES,
            Self::LES_Rw_Mw(_, _) => Mnemonic::LES,
            Self::LFS_Rd_Md(_, _) => Mnemonic::LFS,
            Self::LFS_Rw_Mw(_, _) => Mnemonic::LFS,
            Self::LGDT_MpO16(_) => Mnemonic::LGDT,
            Self::LGDT_MpO32(_) => Mnemonic::LGDT,
            Self::LGS_Rd_Md(_, _) => Mnemonic::LGS,
            Self::LGS_Rw_Mw(_, _) => Mnemonic::LGS,
            Self::LIDT_MpO16(_) => Mnemonic::LIDT,
            Self::LIDT_MpO32(_) => Mnemonic::LIDT,
            Self::LLDT_Mw(_) => Mnemonic::LLDT,
            Self::LLDT_Rw(_) => Mnemonic::LLDT,
            Self::LMSW_Mw(_) => Mnemonic::LMSW,
            Self::LMSW_Rw(_) => Mnemonic::LMSW,
            Self::LODSB(_) => Mnemonic::LODS,
            Self::LODSD(_) => Mnemonic::LODS,
            Self::LODSW(_) => Mnemonic::LODS,
            Self::LOOPNZ_Jd(_, _) => Mnemonic::LOOPNZ,
            Self::LOOPNZ_Jw(_, _) => Mnemonic::LOOPNZ,
            Self::LOOPZ_Jd(_, _) => Mnemonic::LOOPZ,
            Self::LOOPZ_Jw(_, _) => Mnemonic::LOOPZ,
            Self::LOOP_Jd(_, _) => Mnemonic::LOOP,
            Self::LOOP_Jw(_, _) => Mnemonic::LOOP,
            Self::LSL_Rd_Mw(_, _) => Mnemonic::LSL,
            Self::LSL_Rd_Rw(_, _) => Mnemonic::LSL,
            Self::LSL_Rw_Mw(_, _) => Mnemonic::LSL,
            Self::LSL_Rw_Rw(_, _) => Mnemonic::LSL,
            Self::LSS_Rd_Md(_, _) => Mnemonic::LSS,
            Self::LSS_Rw_Mw(_, _) => Mnemonic::LSS,
            Self::LTR_Mw(_) => Mnemonic::LTR,
            Self::LTR_Rw(_) => Mnemonic::LTR,
            Self::MOVSB(_) => Mnemonic::MOVS,
            Self::MOVSD(_) => Mnemonic::MOVS,
            Self::MOVSW(_) => Mnemonic::MOVS,
            Self::MOVSX_Rd_Mb(_, _) => Mnemonic::MOVSX,
            Self::MOVSX_Rd_Mw(_, _) => Mnemonic::MOVSX,
            Self::MOVSX_Rd_Rb(_, _) => Mnemonic::MOVSX,
            Self::MOVSX_Rd_Rw(_, _) => Mnemonic::MOVSX,
            Self::MOVSX_Rw_Mb(_, _) => Mnemonic::MOVSX,
            Self::MOVSX_Rw_Rb(_, _) => Mnemonic::MOVSX,
            Self::MOVZX_Rd_Mb(_, _) => Mnemonic::MOVZX,
            Self::MOVZX_Rd_Mw(_, _) => Mnemonic::MOVZX,
            Self::MOVZX_Rd_Rb(_, _) => Mnemonic::MOVZX,
            Self::MOVZX_Rd_Rw(_, _) => Mnemonic::MOVZX,
            Self::MOVZX_Rw_Mb(_, _) => Mnemonic::MOVZX,
            Self::MOVZX_Rw_Rb(_, _) => Mnemonic::MOVZX,
            Self::MOV_Cr_Rd(_, _) => Mnemonic::MOV,
            Self::MOV_Dr_Rd(_, _) => Mnemonic::MOV,
            Self::MOV_Mb_Ib(_, _) => Mnemonic::MOV,
            Self::MOV_Mb_Rb(_, _) => Mnemonic::MOV,
            Self::MOV_Md_Id(_, _) => Mnemonic::MOV,
            Self::MOV_Md_Rd(_, _) => Mnemonic::MOV,
            Self::MOV_Md_Sr(_, _) => Mnemonic::MOV,
            Self::MOV_Mw_Iw(_, _) => Mnemonic::MOV,
            Self::MOV_Mw_Rw(_, _) => Mnemonic::MOV,
            Self::MOV_Mw_Sr(_, _) => Mnemonic::MOV,
            Self::MOV_Rb_Ib(_, _) => Mnemonic::MOV,
            Self::MOV_Rb_Mb(_, _) => Mnemonic::MOV,
            Self::MOV_Rb_Rb(_, _) => Mnemonic::MOV,
            Self::MOV_Rd_Cr(_, _) => Mnemonic::MOV,
            Self::MOV_Rd_Dr(_, _) => Mnemonic::MOV,
            Self::MOV_Rd_Id(_, _) => Mnemonic::MOV,
            Self::MOV_Rd_Md(_, _) => Mnemonic::MOV,
            Self::MOV_Rd_Rd(_, _) => Mnemonic::MOV,
            Self::MOV_Rd_Sr(_, _) => Mnemonic::MOV,
            Self::MOV_Rw_Iw(_, _) => Mnemonic::MOV,
            Self::MOV_Rw_Mw(_, _) => Mnemonic::MOV,
            Self::MOV_Rw_Rw(_, _) => Mnemonic::MOV,
            Self::MOV_Rw_Sr(_, _) => Mnemonic::MOV,
            Self::MOV_Sr_Md(_, _) => Mnemonic::MOV,
            Self::MOV_Sr_Mw(_, _) => Mnemonic::MOV,
            Self::MOV_Sr_Rd(_, _) => Mnemonic::MOV,
            Self::MOV_Sr_Rw(_, _) => Mnemonic::MOV,
            Self::MUL_Mb(_) => Mnemonic::MUL,
            Self::MUL_Md(_) => Mnemonic::MUL,
            Self::MUL_Mw(_) => Mnemonic::MUL,
            Self::MUL_Rb(_) => Mnemonic::MUL,
            Self::MUL_Rd(_) => Mnemonic::MUL,
            Self::MUL_Rw(_) => Mnemonic::MUL,
            Self::NEG_Mb(_) => Mnemonic::NEG,
            Self::NEG_Md(_) => Mnemonic::NEG,
            Self::NEG_Mw(_) => Mnemonic::NEG,
            Self::NEG_Rb(_) => Mnemonic::NEG,
            Self::NEG_Rd(_) => Mnemonic::NEG,
            Self::NEG_Rw(_) => Mnemonic::NEG,
            Self::NOP => Mnemonic::NOP,
            Self::NOT_Mb(_) => Mnemonic::NOT,
            Self::NOT_Md(_) => Mnemonic::NOT,
            Self::NOT_Mw(_) => Mnemonic::NOT,
            Self::NOT_Rb(_) => Mnemonic::NOT,
            Self::NOT_Rd(_) => Mnemonic::NOT,
            Self::NOT_Rw(_) => Mnemonic::NOT,
            Self::OR_Mb_Ib(_, _) => Mnemonic::OR,
            Self::OR_Mb_Rb(_, _) => Mnemonic::OR,
            Self::OR_Md_Id(_, _) => Mnemonic::OR,
            Self::OR_Md_Rd(_, _) => Mnemonic::OR,
            Self::OR_Mw_Iw(_, _) => Mnemonic::OR,
            Self::OR_Mw_Rw(_, _) => Mnemonic::OR,
            Self::OR_Rb_Ib(_, _) => Mnemonic::OR,
            Self::OR_Rb_Mb(_, _) => Mnemonic::OR,
            Self::OR_Rb_Rb(_, _) => Mnemonic::OR,
            Self::OR_Rd_Id(_, _) => Mnemonic::OR,
            Self::OR_Rd_Md(_, _) => Mnemonic::OR,
            Self::OR_Rd_Rd(_, _) => Mnemonic::OR,
            Self::OR_Rw_Iw(_, _) => Mnemonic::OR,
            Self::OR_Rw_Mw(_, _) => Mnemonic::OR,
            Self::OR_Rw_Rw(_, _) => Mnemonic::OR,
            Self::OUTSB(_) => Mnemonic::OUTS,
            Self::OUTSD(_) => Mnemonic::OUTS,
            Self::OUTSW(_) => Mnemonic::OUTS,
            Self::OUT_Dx_Ad => Mnemonic::OUT,
            Self::OUT_Dx_Al => Mnemonic::OUT,
            Self::OUT_Dx_Aw => Mnemonic::OUT,
            Self::OUT_Ib_Ad(_) => Mnemonic::OUT,
            Self::OUT_Ib_Al(_) => Mnemonic::OUT,
            Self::OUT_Ib_Aw(_) => Mnemonic::OUT,
            Self::PAUSE => Mnemonic::PAUSE,
            Self::POPA => Mnemonic::POPA,
            Self::POPAD => Mnemonic::POPAD,
            Self::POPCNT_Rd_Md(_, _) => Mnemonic::POPCNT,
            Self::POPCNT_Rd_Rd(_, _) => Mnemonic::POPCNT,
            Self::POPCNT_Rw_Mw(_, _) => Mnemonic::POPCNT,
            Self::POPCNT_Rw_Rw(_, _) => Mnemonic::POPCNT,
            Self::POPF => Mnemonic::POPF,
            Self::POPFD => Mnemonic::POPFD,
            Self::POP_Md(_) => Mnemonic::POP,
            Self::POP_Mw(_) => Mnemonic::POP,
            Self::POP_Rd(_) => Mnemonic::POP,
            Self::POP_Rw(_) => Mnemonic::POP,
            Self::POP_SrO16(_) => Mnemonic::POP,
            Self::POP_SrO32(_) => Mnemonic::POP,
            Self::PUSHA => Mnemonic::PUSHA,
            Self::PUSHAD => Mnemonic::PUSHAD,
            Self::PUSHF => Mnemonic::PUSHF,
            Self::PUSHFD => Mnemonic::PUSHFD,
            Self::PUSH_Id(_) => Mnemonic::PUSH,
            Self::PUSH_Iw(_) => Mnemonic::PUSH,
            Self::PUSH_Md(_) => Mnemonic::PUSH,
            Self::PUSH_Mw(_) => Mnemonic::PUSH,
            Self::PUSH_Rd(_) => Mnemonic::PUSH,
            Self::PUSH_Rw(_) => Mnemonic::PUSH,
            Self::PUSH_SrO16(_) => Mnemonic::PUSH,
            Self::PUSH_SrO32(_) => Mnemonic::PUSH,
            Self::RCL_Mb_Cl(_) => Mnemonic::RCL,
            Self::RCL_Mb_Ib(_, _) => Mnemonic::RCL,
            Self::RCL_Md_Cl(_) => Mnemonic::RCL,
            Self::RCL_Md_Ib(_, _) => Mnemonic::RCL,
            Self::RCL_Mw_Cl(_) => Mnemonic::RCL,
            Self::RCL_Mw_Ib(_, _) => Mnemonic::RCL,
            Self::RCL_Rb_Cl(_) => Mnemonic::RCL,
            Self::RCL_Rb_Ib(_, _) => Mnemonic::RCL,
            Self::RCL_Rd_Cl(_) => Mnemonic::RCL,
            Self::RCL_Rd_Ib(_, _) => Mnemonic::RCL,
            Self::RCL_Rw_Cl(_) => Mnemonic::RCL,
            Self::RCL_Rw_Ib(_, _) => Mnemonic::RCL,
            Self::RCR_Mb_Cl(_) => Mnemonic::RCR,
            Self::RCR_Mb_Ib(_, _) => Mnemonic::RCR,
            Self::RCR_Md_Cl(_) => Mnemonic::RCR,
            Self::RCR_Md_Ib(_, _) => Mnemonic::RCR,
            Self::RCR_Mw_Cl(_) => Mnemonic::RCR,
            Self::RCR_Mw_Ib(_, _) => Mnemonic::RCR,
            Self::RCR_Rb_Cl(_) => Mnemonic::RCR,
            Self::RCR_Rb_Ib(_, _) => Mnemonic::RCR,
            Self::RCR_Rd_Cl(_) => Mnemonic::RCR,
            Self::RCR_Rd_Ib(_, _) => Mnemonic::RCR,
            Self::RCR_Rw_Cl(_) => Mnemonic::RCR,
            Self::RCR_Rw_Ib(_, _) => Mnemonic::RCR,
            Self::RDMSR => Mnemonic::RDMSR,
            Self::RDTSC => Mnemonic::RDTSC,
            Self::RDTSCP => Mnemonic::RDTSCP,
            Self::REPNZ_CMPSB(_) => Mnemonic::CMPS,
            Self::REPNZ_CMPSD(_) => Mnemonic::CMPS,
            Self::REPNZ_CMPSW(_) => Mnemonic::CMPS,
            Self::REPNZ_SCASB(_) => Mnemonic::SCAS,
            Self::REPNZ_SCASD(_) => Mnemonic::SCAS,
            Self::REPNZ_SCASW(_) => Mnemonic::SCAS,
            Self::REPZ_CMPSB(_) => Mnemonic::CMPS,
            Self::REPZ_CMPSD(_) => Mnemonic::CMPS,
            Self::REPZ_CMPSW(_) => Mnemonic::CMPS,
            Self::REPZ_SCASB(_) => Mnemonic::SCAS,
            Self::REPZ_SCASD(_) => Mnemonic::SCAS,
            Self::REPZ_SCASW(_) => Mnemonic::SCAS,
            Self::REP_INSB(_) => Mnemonic::INS,
            Self::REP_INSD(_) => Mnemonic::INS,
            Self::REP_INSW(_) => Mnemonic::INS,
            Self::REP_LODSB(_) => Mnemonic::LODS,
            Self::REP_LODSD(_) => Mnemonic::LODS,
            Self::REP_LODSW(_) => Mnemonic::LODS,
            Self::REP_MOVSB(_) => Mnemonic::MOVS,
            Self::REP_MOVSD(_) => Mnemonic::MOVS,
            Self::REP_MOVSW(_) => Mnemonic::MOVS,
            Self::REP_OUTSB(_) => Mnemonic::OUTS,
            Self::REP_OUTSD(_) => Mnemonic::OUTS,
            Self::REP_OUTSW(_) => Mnemonic::OUTS,
            Self::REP_STOSB(_) => Mnemonic::STOS,
            Self::REP_STOSD(_) => Mnemonic::STOS,
            Self::REP_STOSW(_) => Mnemonic::STOS,
            Self::RETF_D16(_) => Mnemonic::RETF,
            Self::RETF_D32(_) => Mnemonic::RETF,
            Self::RET_D16(_) => Mnemonic::RET,
            Self::RET_D32(_) => Mnemonic::RET,
            Self::ROL_Mb_Cl(_) => Mnemonic::ROL,
            Self::ROL_Mb_Ib(_, _) => Mnemonic::ROL,
            Self::ROL_Md_Cl(_) => Mnemonic::ROL,
            Self::ROL_Md_Ib(_, _) => Mnemonic::ROL,
            Self::ROL_Mw_Cl(_) => Mnemonic::ROL,
            Self::ROL_Mw_Ib(_, _) => Mnemonic::ROL,
            Self::ROL_Rb_Cl(_) => Mnemonic::ROL,
            Self::ROL_Rb_Ib(_, _) => Mnemonic::ROL,
            Self::ROL_Rd_Cl(_) => Mnemonic::ROL,
            Self::ROL_Rd_Ib(_, _) => Mnemonic::ROL,
            Self::ROL_Rw_Cl(_) => Mnemonic::ROL,
            Self::ROL_Rw_Ib(_, _) => Mnemonic::ROL,
            Self::ROR_Mb_Cl(_) => Mnemonic::ROR,
            Self::ROR_Mb_Ib(_, _) => Mnemonic::ROR,
            Self::ROR_Md_Cl(_) => Mnemonic::ROR,
            Self::ROR_Md_Ib(_, _) => Mnemonic::ROR,
            Self::ROR_Mw_Cl(_) => Mnemonic::ROR,
            Self::ROR_Mw_Ib(_, _) => Mnemonic::ROR,
            Self::ROR_Rb_Cl(_) => Mnemonic::ROR,
            Self::ROR_Rb_Ib(_, _) => Mnemonic::ROR,
            Self::ROR_Rd_Cl(_) => Mnemonic::ROR,
            Self::ROR_Rd_Ib(_, _) => Mnemonic::ROR,
            Self::ROR_Rw_Cl(_) => Mnemonic::ROR,
            Self::ROR_Rw_Ib(_, _) => Mnemonic::ROR,
            Self::RSM => Mnemonic::RSM,
            Self::SAHF => Mnemonic::SAHF,
            Self::SALC => Mnemonic::SALC,
            Self::SAR_Mb_Cl(_) => Mnemonic::SAR,
            Self::SAR_Mb_Ib(_, _) => Mnemonic::SAR,
            Self::SAR_Md_Cl(_) => Mnemonic::SAR,
            Self::SAR_Md_Ib(_, _) => Mnemonic::SAR,
            Self::SAR_Mw_Cl(_) => Mnemonic::SAR,
            Self::SAR_Mw_Ib(_, _) => Mnemonic::SAR,
            Self::SAR_Rb_Cl(_) => Mnemonic::SAR,
            Self::SAR_Rb_Ib(_, _) => Mnemonic::SAR,
            Self::SAR_Rd_Cl(_) => Mnemonic::SAR,
            Self::SAR_Rd_Ib(_, _) => Mnemonic::SAR,
            Self::SAR_Rw_Cl(_) => Mnemonic::SAR,
            Self::SAR_Rw_Ib(_, _) => Mnemonic::SAR,
            Self::SBB_Mb_Ib(_, _) => Mnemonic::SBB,
            Self::SBB_Mb_Rb(_, _) => Mnemonic::SBB,
            Self::SBB_Md_Id(_, _) => Mnemonic::SBB,
            Self::SBB_Md_Rd(_, _) => Mnemonic::SBB,
            Self::SBB_Mw_Iw(_, _) => Mnemonic::SBB,
            Self::SBB_Mw_Rw(_, _) => Mnemonic::SBB,
            Self::SBB_Rb_Ib(_, _) => Mnemonic::SBB,
            Self::SBB_Rb_Mb(_, _) => Mnemonic::SBB,
            Self::SBB_Rb_Rb(_, _) => Mnemonic::SBB,
            Self::SBB_Rd_Id(_, _) => Mnemonic::SBB,
            Self::SBB_Rd_Md(_, _) => Mnemonic::SBB,
            Self::SBB_Rd_Rd(_, _) => Mnemonic::SBB,
            Self::SBB_Rw_Iw(_, _) => Mnemonic::SBB,
            Self::SBB_Rw_Mw(_, _) => Mnemonic::SBB,
            Self::SBB_Rw_Rw(_, _) => Mnemonic::SBB,
            Self::SCASB(_) => Mnemonic::SCAS,
            Self::SCASD(_) => Mnemonic::SCAS,
            Self::SCASW(_) => Mnemonic::SCAS,
            Self::SETCC_Mb(_, _) => Mnemonic::SETCC,
            Self::SETCC_Rb(_, _) => Mnemonic::SETCC,
            Self::SGDT_Mp(_) => Mnemonic::SGDT,
            Self::SHLD_Md_Rd_Cl(_, _) => Mnemonic::SHLD,
            Self::SHLD_Md_Rd_Ib(_, _, _) => Mnemonic::SHLD,
            Self::SHLD_Mw_Rw_Cl(_, _) => Mnemonic::SHLD,
            Self::SHLD_Mw_Rw_Ib(_, _, _) => Mnemonic::SHLD,
            Self::SHLD_Rd_Rd_Cl(_, _) => Mnemonic::SHLD,
            Self::SHLD_Rd_Rd_Ib(_, _, _) => Mnemonic::SHLD,
            Self::SHLD_Rw_Rw_Cl(_, _) => Mnemonic::SHLD,
            Self::SHLD_Rw_Rw_Ib(_, _, _) => Mnemonic::SHLD,
            Self::SHL_Mb_Cl(_) => Mnemonic::SHL,
            Self::SHL_Mb_Ib(_, _) => Mnemonic::SHL,
            Self::SHL_Md_Cl(_) => Mnemonic::SHL,
            Self::SHL_Md_Ib(_, _) => Mnemonic::SHL,
            Self::SHL_Mw_Cl(_) => Mnemonic::SHL,
            Self::SHL_Mw_Ib(_, _) => Mnemonic::SHL,
            Self::SHL_Rb_Cl(_) => Mnemonic::SHL,
            Self::SHL_Rb_Ib(_, _) => Mnemonic::SHL,
            Self::SHL_Rd_Cl(_) => Mnemonic::SHL,
            Self::SHL_Rd_Ib(_, _) => Mnemonic::SHL,
            Self::SHL_Rw_Cl(_) => Mnemonic::SHL,
            Self::SHL_Rw_Ib(_, _) => Mnemonic::SHL,
            Self::SHRD_Md_Rd_Cl(_, _) => Mnemonic::SHRD,
            Self::SHRD_Md_Rd_Ib(_, _, _) => Mnemonic::SHRD,
            Self::SHRD_Mw_Rw_Cl(_, _) => Mnemonic::SHRD,
            Self::SHRD_Mw_Rw_Ib(_, _, _) => Mnemonic::SHRD,
            Self::SHRD_Rd_Rd_Cl(_, _) => Mnemonic::SHRD,
            Self::SHRD_Rd_Rd_Ib(_, _, _) => Mnemonic::SHRD,
            Self::SHRD_Rw_Rw_Cl(_, _) => Mnemonic::SHRD,
            Self::SHRD_Rw_Rw_Ib(_, _, _) => Mnemonic::SHRD,
            Self::SHR_Mb_Cl(_) => Mnemonic::SHR,
            Self::SHR_Mb_Ib(_, _) => Mnemonic::SHR,
            Self::SHR_Md_Cl(_) => Mnemonic::SHR,
            Self::SHR_Md_Ib(_, _) => Mnemonic::SHR,
            Self::SHR_Mw_Cl(_) => Mnemonic::SHR,
            Self::SHR_Mw_Ib(_, _) => Mnemonic::SHR,
            Self::SHR_Rb_Cl(_) => Mnemonic::SHR,
            Self::SHR_Rb_Ib(_, _) => Mnemonic::SHR,
            Self::SHR_Rd_Cl(_) => Mnemonic::SHR,
            Self::SHR_Rd_Ib(_, _) => Mnemonic::SHR,
            Self::SHR_Rw_Cl(_) => Mnemonic::SHR,
            Self::SHR_Rw_Ib(_, _) => Mnemonic::SHR,
            Self::SIDT_Mp(_) => Mnemonic::SIDT,
            Self::SIMD(_) => Mnemonic::SIMD,
            Self::SIMD_M(_, _, _, _) => Mnemonic::SIMD,
            Self::SIMD_R(_, _, _, _) => Mnemonic::SIMD,
            Self::SIMD_S(_, _, _, _) => Mnemonic::SIMD,
            Self::SLDT_Md(_) => Mnemonic::SLDT,
            Self::SLDT_Mw(_) => Mnemonic::SLDT,
            Self::SLDT_Rd(_) => Mnemonic::SLDT,
            Self::SLDT_Rw(_) => Mnemonic::SLDT,
            Self::SMSW_Mw(_) => Mnemonic::SMSW,
            Self::SMSW_Rw(_) => Mnemonic::SMSW,
            Self::STC => Mnemonic::STC,
            Self::STD => Mnemonic::STD,
            Self::STI => Mnemonic::STI,
            Self::STOSB(_) => Mnemonic::STOS,
            Self::STOSD(_) => Mnemonic::STOS,
            Self::STOSW(_) => Mnemonic::STOS,
            Self::STR_Md(_) => Mnemonic::STR,
            Self::STR_Mw(_) => Mnemonic::STR,
            Self::STR_Rd(_) => Mnemonic::STR,
            Self::STR_Rw(_) => Mnemonic::STR,
            Self::SUB_Mb_Ib(_, _) => Mnemonic::SUB,
            Self::SUB_Mb_Rb(_, _) => Mnemonic::SUB,
            Self::SUB_Md_Id(_, _) => Mnemonic::SUB,
            Self::SUB_Md_Rd(_, _) => Mnemonic::SUB,
            Self::SUB_Mw_Iw(_, _) => Mnemonic::SUB,
            Self::SUB_Mw_Rw(_, _) => Mnemonic::SUB,
            Self::SUB_Rb_Ib(_, _) => Mnemonic::SUB,
            Self::SUB_Rb_Mb(_, _) => Mnemonic::SUB,
            Self::SUB_Rb_Rb(_, _) => Mnemonic::SUB,
            Self::SUB_Rd_Id(_, _) => Mnemonic::SUB,
            Self::SUB_Rd_Md(_, _) => Mnemonic::SUB,
            Self::SUB_Rd_Rd(_, _) => Mnemonic::SUB,
            Self::SUB_Rw_Iw(_, _) => Mnemonic::SUB,
            Self::SUB_Rw_Mw(_, _) => Mnemonic::SUB,
            Self::SUB_Rw_Rw(_, _) => Mnemonic::SUB,
            Self::SWAPGS => Mnemonic::SWAPGS,
            Self::SYSCALL => Mnemonic::SYSCALL,
            Self::SYSENTER => Mnemonic::SYSENTER,
            Self::SYSEXIT => Mnemonic::SYSEXIT,
            Self::SYSRET => Mnemonic::SYSRET,
            Self::TEST_Mb_Ib(_, _) => Mnemonic::TEST,
            Self::TEST_Mb_Rb(_, _) => Mnemonic::TEST,
            Self::TEST_Md_Id(_, _) => Mnemonic::TEST,
            Self::TEST_Md_Rd(_, _) => Mnemonic::TEST,
            Self::TEST_Mw_Iw(_, _) => Mnemonic::TEST,
            Self::TEST_Mw_Rw(_, _) => Mnemonic::TEST,
            Self::TEST_Rb_Ib(_, _) => Mnemonic::TEST,
            Self::TEST_Rb_Rb(_, _) => Mnemonic::TEST,
            Self::TEST_Rd_Id(_, _) => Mnemonic::TEST,
            Self::TEST_Rd_Rd(_, _) => Mnemonic::TEST,
            Self::TEST_Rw_Iw(_, _) => Mnemonic::TEST,
            Self::TEST_Rw_Rw(_, _) => Mnemonic::TEST,
            Self::UD => Mnemonic::UD,
            Self::VERR_Mw(_) => Mnemonic::VERR,
            Self::VERR_Rw(_) => Mnemonic::VERR,
            Self::VERW_Mw(_) => Mnemonic::VERW,
            Self::VERW_Rw(_) => Mnemonic::VERW,
            Self::WAIT => Mnemonic::WAIT,
            Self::WBINVD => Mnemonic::WBINVD,
            Self::WRMSR => Mnemonic::WRMSR,
            Self::XADD_Mb_Rb(_, _) => Mnemonic::XADD,
            Self::XADD_Md_Rd(_, _) => Mnemonic::XADD,
            Self::XADD_Mw_Rw(_, _) => Mnemonic::XADD,
            Self::XADD_Rb_Rb(_, _) => Mnemonic::XADD,
            Self::XADD_Rd_Rd(_, _) => Mnemonic::XADD,
            Self::XADD_Rw_Rw(_, _) => Mnemonic::XADD,
            Self::XCHG_Mb_Rb(_, _) => Mnemonic::XCHG,
            Self::XCHG_Md_Rd(_, _) => Mnemonic::XCHG,
            Self::XCHG_Mw_Rw(_, _) => Mnemonic::XCHG,
            Self::XCHG_Rb_Rb(_, _) => Mnemonic::XCHG,
            Self::XCHG_Rd_Rd(_, _) => Mnemonic::XCHG,
            Self::XCHG_Rw_Rw(_, _) => Mnemonic::XCHG,
            Self::XGETBV => Mnemonic::XGETBV,
            Self::XLAT_Mb(_) => Mnemonic::XLAT,
            Self::XOR_Mb_Ib(_, _) => Mnemonic::XOR,
            Self::XOR_Mb_Rb(_, _) => Mnemonic::XOR,
            Self::XOR_Md_Id(_, _) => Mnemonic::XOR,
            Self::XOR_Md_Rd(_, _) => Mnemonic::XOR,
            Self::XOR_Mw_Iw(_, _) => Mnemonic::XOR,
            Self::XOR_Mw_Rw(_, _) => Mnemonic::XOR,
            Self::XOR_Rb_Ib(_, _) => Mnemonic::XOR,
            Self::XOR_Rb_Mb(_, _) => Mnemonic::XOR,
            Self::XOR_Rb_Rb(_, _) => Mnemonic::XOR,
            Self::XOR_Rd_Id(_, _) => Mnemonic::XOR,
            Self::XOR_Rd_Md(_, _) => Mnemonic::XOR,
            Self::XOR_Rd_Rd(_, _) => Mnemonic::XOR,
            Self::XOR_Rw_Iw(_, _) => Mnemonic::XOR,
            Self::XOR_Rw_Mw(_, _) => Mnemonic::XOR,
            Self::XOR_Rw_Rw(_, _) => Mnemonic::XOR,
            Self::XSETBV => Mnemonic::XSETBV,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::AAA => "AAA",
            Self::AAD_Ib(_) => "AAD_Ib",
            Self::AAM_Ib(_) => "AAM_Ib",
            Self::AAS => "AAS",
            Self::ADC_Mb_Ib(_, _) => "ADC_Mb_Ib",
            Self::ADC_Mb_Rb(_, _) => "ADC_Mb_Rb",
            Self::ADC_Md_Id(_, _) => "ADC_Md_Id",
            Self::ADC_Md_Rd(_, _) => "ADC_Md_Rd",
            Self::ADC_Mw_Iw(_, _) => "ADC_Mw_Iw",
            Self::ADC_Mw_Rw(_, _) => "ADC_Mw_Rw",
            Self::ADC_Rb_Ib(_, _) => "ADC_Rb_Ib",
            Self::ADC_Rb_Mb(_, _) => "ADC_Rb_Mb",
            Self::ADC_Rb_Rb(_, _) => "ADC_Rb_Rb",
            Self::ADC_Rd_Id(_, _) => "ADC_Rd_Id",
            Self::ADC_Rd_Md(_, _) => "ADC_Rd_Md",
            Self::ADC_Rd_Rd(_, _) => "ADC_Rd_Rd",
            Self::ADC_Rw_Iw(_, _) => "ADC_Rw_Iw",
            Self::ADC_Rw_Mw(_, _) => "ADC_Rw_Mw",
            Self::ADC_Rw_Rw(_, _) => "ADC_Rw_Rw",
            Self::ADD_Mb_Ib(_, _) => "ADD_Mb_Ib",
            Self::ADD_Mb_Rb(_, _) => "ADD_Mb_Rb",
            Self::ADD_Md_Id(_, _) => "ADD_Md_Id",
            Self::ADD_Md_Rd(_, _) => "ADD_Md_Rd",
            Self::ADD_Mw_Iw(_, _) => "ADD_Mw_Iw",
            Self::ADD_Mw_Rw(_, _) => "ADD_Mw_Rw",
            Self::ADD_Rb_Ib(_, _) => "ADD_Rb_Ib",
            Self::ADD_Rb_Mb(_, _) => "ADD_Rb_Mb",
            Self::ADD_Rb_Rb(_, _) => "ADD_Rb_Rb",
            Self::ADD_Rd_Id(_, _) => "ADD_Rd_Id",
            Self::ADD_Rd_Md(_, _) => "ADD_Rd_Md",
            Self::ADD_Rd_Rd(_, _) => "ADD_Rd_Rd",
            Self::ADD_Rw_Iw(_, _) => "ADD_Rw_Iw",
            Self::ADD_Rw_Mw(_, _) => "ADD_Rw_Mw",
            Self::ADD_Rw_Rw(_, _) => "ADD_Rw_Rw",
            Self::AND_Mb_Ib(_, _) => "AND_Mb_Ib",
            Self::AND_Mb_Rb(_, _) => "AND_Mb_Rb",
            Self::AND_Md_Id(_, _) => "AND_Md_Id",
            Self::AND_Md_Rd(_, _) => "AND_Md_Rd",
            Self::AND_Mw_Iw(_, _) => "AND_Mw_Iw",
            Self::AND_Mw_Rw(_, _) => "AND_Mw_Rw",
            Self::AND_Rb_Ib(_, _) => "AND_Rb_Ib",
            Self::AND_Rb_Mb(_, _) => "AND_Rb_Mb",
            Self::AND_Rb_Rb(_, _) => "AND_Rb_Rb",
            Self::AND_Rd_Id(_, _) => "AND_Rd_Id",
            Self::AND_Rd_Md(_, _) => "AND_Rd_Md",
            Self::AND_Rd_Rd(_, _) => "AND_Rd_Rd",
            Self::AND_Rw_Iw(_, _) => "AND_Rw_Iw",
            Self::AND_Rw_Mw(_, _) => "AND_Rw_Mw",
            Self::AND_Rw_Rw(_, _) => "AND_Rw_Rw",
            Self::ARPL_Mw_Rw(_, _) => "ARPL_Mw_Rw",
            Self::ARPL_Rw_Rw(_, _) => "ARPL_Rw_Rw",
            Self::BOUND_Rd_Md(_, _) => "BOUND_Rd_Md",
            Self::BOUND_Rw_Mw(_, _) => "BOUND_Rw_Mw",
            Self::BSF_Rd_Md(_, _) => "BSF_Rd_Md",
            Self::BSF_Rd_Rd(_, _) => "BSF_Rd_Rd",
            Self::BSF_Rw_Mw(_, _) => "BSF_Rw_Mw",
            Self::BSF_Rw_Rw(_, _) => "BSF_Rw_Rw",
            Self::BSR_Rd_Md(_, _) => "BSR_Rd_Md",
            Self::BSR_Rd_Rd(_, _) => "BSR_Rd_Rd",
            Self::BSR_Rw_Mw(_, _) => "BSR_Rw_Mw",
            Self::BSR_Rw_Rw(_, _) => "BSR_Rw_Rw",
            Self::BSWAP_Rd(_) => "BSWAP_Rd",
            Self::BTC_Md_Ib(_, _) => "BTC_Md_Ib",
            Self::BTC_Md_Rd(_, _) => "BTC_Md_Rd",
            Self::BTC_Mw_Ib(_, _) => "BTC_Mw_Ib",
            Self::BTC_Mw_Rw(_, _) => "BTC_Mw_Rw",
            Self::BTC_Rd_Ib(_, _) => "BTC_Rd_Ib",
            Self::BTC_Rd_Rd(_, _) => "BTC_Rd_Rd",
            Self::BTC_Rw_Ib(_, _) => "BTC_Rw_Ib",
            Self::BTC_Rw_Rw(_, _) => "BTC_Rw_Rw",
            Self::BTR_Md_Ib(_, _) => "BTR_Md_Ib",
            Self::BTR_Md_Rd(_, _) => "BTR_Md_Rd",
            Self::BTR_Mw_Ib(_, _) => "BTR_Mw_Ib",
            Self::BTR_Mw_Rw(_, _) => "BTR_Mw_Rw",
            Self::BTR_Rd_Ib(_, _) => "BTR_Rd_Ib",
            Self::BTR_Rd_Rd(_, _) => "BTR_Rd_Rd",
            Self::BTR_Rw_Ib(_, _) => "BTR_Rw_Ib",
            Self::BTR_Rw_Rw(_, _) => "BTR_Rw_Rw",
            Self::BTS_Md_Ib(_, _) => "BTS_Md_Ib",
            Self::BTS_Md_Rd(_, _) => "BTS_Md_Rd",
            Self::BTS_Mw_Ib(_, _) => "BTS_Mw_Ib",
            Self::BTS_Mw_Rw(_, _) => "BTS_Mw_Rw",
            Self::BTS_Rd_Ib(_, _) => "BTS_Rd_Ib",
            Self::BTS_Rd_Rd(_, _) => "BTS_Rd_Rd",
            Self::BTS_Rw_Ib(_, _) => "BTS_Rw_Ib",
            Self::BTS_Rw_Rw(_, _) => "BTS_Rw_Rw",
            Self::BT_Md_Ib(_, _) => "BT_Md_Ib",
            Self::BT_Md_Rd(_, _) => "BT_Md_Rd",
            Self::BT_Mw_Ib(_, _) => "BT_Mw_Ib",
            Self::BT_Mw_Rw(_, _) => "BT_Mw_Rw",
            Self::BT_Rd_Ib(_, _) => "BT_Rd_Ib",
            Self::BT_Rd_Rd(_, _) => "BT_Rd_Rd",
            Self::BT_Rw_Ib(_, _) => "BT_Rw_Ib",
            Self::BT_Rw_Rw(_, _) => "BT_Rw_Rw",
            Self::CALLF_ApO16(_, _) => "CALLF_ApO16",
            Self::CALLF_ApO32(_, _) => "CALLF_ApO32",
            Self::CALLF_MpO16(_) => "CALLF_MpO16",
            Self::CALLF_MpO32(_) => "CALLF_MpO32",
            Self::CALL_Jd(_) => "CALL_Jd",
            Self::CALL_Jw(_) => "CALL_Jw",
            Self::CALL_Md(_) => "CALL_Md",
            Self::CALL_Mw(_) => "CALL_Mw",
            Self::CALL_Rd(_) => "CALL_Rd",
            Self::CALL_Rw(_) => "CALL_Rw",
            Self::CDQ => "CDQ",
            Self::CLC => "CLC",
            Self::CLD => "CLD",
            Self::CLI => "CLI",
            Self::CLTS => "CLTS",
            Self::CMC => "CMC",
            Self::CMOV_Rd_Md(_, _, _) => "CMOV_Rd_Md",
            Self::CMOV_Rd_Rd(_, _, _) => "CMOV_Rd_Rd",
            Self::CMOV_Rw_Mw(_, _, _) => "CMOV_Rw_Mw",
            Self::CMOV_Rw_Rw(_, _, _) => "CMOV_Rw_Rw",
            Self::CMPSB(_) => "CMPSB",
            Self::CMPSD(_) => "CMPSD",
            Self::CMPSW(_) => "CMPSW",
            Self::CMPXCHG_Mb_Rb(_, _) => "CMPXCHG_Mb_Rb",
            Self::CMPXCHG_Md_Rd(_, _) => "CMPXCHG_Md_Rd",
            Self::CMPXCHG_Mw_Rw(_, _) => "CMPXCHG_Mw_Rw",
            Self::CMPXCHG_Rb_Rb(_, _) => "CMPXCHG_Rb_Rb",
            Self::CMPXCHG_Rd_Rd(_, _) => "CMPXCHG_Rd_Rd",
            Self::CMPXCHG_Rw_Rw(_, _) => "CMPXCHG_Rw_Rw",
            Self::CMP_Mb_Ib(_, _) => "CMP_Mb_Ib",
            Self::CMP_Mb_Rb(_, _) => "CMP_Mb_Rb",
            Self::CMP_Md_Id(_, _) => "CMP_Md_Id",
            Self::CMP_Md_Rd(_, _) => "CMP_Md_Rd",
            Self::CMP_Mw_Iw(_, _) => "CMP_Mw_Iw",
            Self::CMP_Mw_Rw(_, _) => "CMP_Mw_Rw",
            Self::CMP_Rb_Ib(_, _) => "CMP_Rb_Ib",
            Self::CMP_Rb_Mb(_, _) => "CMP_Rb_Mb",
            Self::CMP_Rb_Rb(_, _) => "CMP_Rb_Rb",
            Self::CMP_Rd_Id(_, _) => "CMP_Rd_Id",
            Self::CMP_Rd_Md(_, _) => "CMP_Rd_Md",
            Self::CMP_Rd_Rd(_, _) => "CMP_Rd_Rd",
            Self::CMP_Rw_Iw(_, _) => "CMP_Rw_Iw",
            Self::CMP_Rw_Mw(_, _) => "CMP_Rw_Mw",
            Self::CMP_Rw_Rw(_, _) => "CMP_Rw_Rw",
            Self::CPUID => "CPUID",
            Self::CWD => "CWD",
            Self::DAA => "DAA",
            Self::DAS => "DAS",
            Self::DEC_Mb(_) => "DEC_Mb",
            Self::DEC_Md(_) => "DEC_Md",
            Self::DEC_Mw(_) => "DEC_Mw",
            Self::DEC_Rb(_) => "DEC_Rb",
            Self::DEC_Rd(_) => "DEC_Rd",
            Self::DEC_Rw(_) => "DEC_Rw",
            Self::DIV_Mb(_) => "DIV_Mb",
            Self::DIV_Md(_) => "DIV_Md",
            Self::DIV_Mw(_) => "DIV_Mw",
            Self::DIV_Rb(_) => "DIV_Rb",
            Self::DIV_Rd(_) => "DIV_Rd",
            Self::DIV_Rw(_) => "DIV_Rw",
            Self::ENTER_Iw_Ib(_, _) => "ENTER_Iw_Ib",
            Self::ESC_Mb(_, _, _) => "ESC_Mb",
            Self::ESC_Rb(_, _, _) => "ESC_Rb",
            Self::HLT => "HLT",
            Self::ICEBP => "ICEBP",
            Self::IDIV_Mb(_) => "IDIV_Mb",
            Self::IDIV_Md(_) => "IDIV_Md",
            Self::IDIV_Mw(_) => "IDIV_Mw",
            Self::IDIV_Rb(_) => "IDIV_Rb",
            Self::IDIV_Rd(_) => "IDIV_Rd",
            Self::IDIV_Rw(_) => "IDIV_Rw",
            Self::IMUL_Mb(_) => "IMUL_Mb",
            Self::IMUL_Md(_) => "IMUL_Md",
            Self::IMUL_Mw(_) => "IMUL_Mw",
            Self::IMUL_Rb(_) => "IMUL_Rb",
            Self::IMUL_Rd(_) => "IMUL_Rd",
            Self::IMUL_Rd_Md(_, _) => "IMUL_Rd_Md",
            Self::IMUL_Rd_Md_Id(_, _, _) => "IMUL_Rd_Md_Id",
            Self::IMUL_Rd_Rd(_, _) => "IMUL_Rd_Rd",
            Self::IMUL_Rd_Rd_Id(_, _, _) => "IMUL_Rd_Rd_Id",
            Self::IMUL_Rw(_) => "IMUL_Rw",
            Self::IMUL_Rw_Mw(_, _) => "IMUL_Rw_Mw",
            Self::IMUL_Rw_Mw_Iw(_, _, _) => "IMUL_Rw_Mw_Iw",
            Self::IMUL_Rw_Rw(_, _) => "IMUL_Rw_Rw",
            Self::IMUL_Rw_Rw_Iw(_, _, _) => "IMUL_Rw_Rw_Iw",
            Self::INC_Mb(_) => "INC_Mb",
            Self::INC_Md(_) => "INC_Md",
            Self::INC_Mw(_) => "INC_Mw",
            Self::INC_Rb(_) => "INC_Rb",
            Self::INC_Rd(_) => "INC_Rd",
            Self::INC_Rw(_) => "INC_Rw",
            Self::INSB(_) => "INSB",
            Self::INSD(_) => "INSD",
            Self::INSW(_) => "INSW",
            Self::INT3 => "INT3",
            Self::INTO => "INTO",
            Self::INT_Ib(_) => "INT_Ib",
            Self::INVD => "INVD",
            Self::INVLPG_Mb(_) => "INVLPG_Mb",
            Self::INVLPG_Md(_) => "INVLPG_Md",
            Self::INVLPG_Mw(_) => "INVLPG_Mw",
            Self::INVLPG_Rb(_) => "INVLPG_Rb",
            Self::INVLPG_Rd(_) => "INVLPG_Rd",
            Self::INVLPG_Rw(_) => "INVLPG_Rw",
            Self::IN_Ad_Dx => "IN_Ad_Dx",
            Self::IN_Ad_Ib(_) => "IN_Ad_Ib",
            Self::IN_Al_Dx => "IN_Al_Dx",
            Self::IN_Al_Ib(_) => "IN_Al_Ib",
            Self::IN_Aw_Dx => "IN_Aw_Dx",
            Self::IN_Aw_Ib(_) => "IN_Aw_Ib",
            Self::IRET => "IRET",
            Self::IRETD => "IRETD",
            Self::JCC_Jd(_, _) => "JCC_Jd",
            Self::JCC_Jw(_, _) => "JCC_Jw",
            Self::JCXZ_Jd(_) => "JCXZ_Jd",
            Self::JCXZ_Jw(_) => "JCXZ_Jw",
            Self::JECXZ_Jd(_) => "JECXZ_Jd",
            Self::JECXZ_Jw(_) => "JECXZ_Jw",
            Self::JMPF_ApO16(_, _) => "JMPF_ApO16",
            Self::JMPF_ApO32(_, _) => "JMPF_ApO32",
            Self::JMPF_MpO16(_) => "JMPF_MpO16",
            Self::JMPF_MpO32(_) => "JMPF_MpO32",
            Self::JMP_Jd(_) => "JMP_Jd",
            Self::JMP_Jw(_) => "JMP_Jw",
            Self::JMP_Md(_) => "JMP_Md",
            Self::JMP_Mw(_) => "JMP_Mw",
            Self::JMP_Rd(_) => "JMP_Rd",
            Self::JMP_Rw(_) => "JMP_Rw",
            Self::LAHF => "LAHF",
            Self::LAR_Rd_Mw(_, _) => "LAR_Rd_Mw",
            Self::LAR_Rd_Rw(_, _) => "LAR_Rd_Rw",
            Self::LAR_Rw_Mw(_, _) => "LAR_Rw_Mw",
            Self::LAR_Rw_Rw(_, _) => "LAR_Rw_Rw",
            Self::LDS_Rd_Md(_, _) => "LDS_Rd_Md",
            Self::LDS_Rw_Mw(_, _) => "LDS_Rw_Mw",
            Self::LEAVE => "LEAVE",
            Self::LEA_Rd_Md(_, _) => "LEA_Rd_Md",
            Self::LEA_Rw_Mw(_, _) => "LEA_Rw_Mw",
            Self::LES_Rd_Md(_, _) => "LES_Rd_Md",
            Self::LES_Rw_Mw(_, _) => "LES_Rw_Mw",
            Self::LFS_Rd_Md(_, _) => "LFS_Rd_Md",
            Self::LFS_Rw_Mw(_, _) => "LFS_Rw_Mw",
            Self::LGDT_MpO16(_) => "LGDT_MpO16",
            Self::LGDT_MpO32(_) => "LGDT_MpO32",
            Self::LGS_Rd_Md(_, _) => "LGS_Rd_Md",
            Self::LGS_Rw_Mw(_, _) => "LGS_Rw_Mw",
            Self::LIDT_MpO16(_) => "LIDT_MpO16",
            Self::LIDT_MpO32(_) => "LIDT_MpO32",
            Self::LLDT_Mw(_) => "LLDT_Mw",
            Self::LLDT_Rw(_) => "LLDT_Rw",
            Self::LMSW_Mw(_) => "LMSW_Mw",
            Self::LMSW_Rw(_) => "LMSW_Rw",
            Self::LODSB(_) => "LODSB",
            Self::LODSD(_) => "LODSD",
            Self::LODSW(_) => "LODSW",
            Self::LOOPNZ_Jd(_, _) => "LOOPNZ_Jd",
            Self::LOOPNZ_Jw(_, _) => "LOOPNZ_Jw",
            Self::LOOPZ_Jd(_, _) => "LOOPZ_Jd",
            Self::LOOPZ_Jw(_, _) => "LOOPZ_Jw",
            Self::LOOP_Jd(_, _) => "LOOP_Jd",
            Self::LOOP_Jw(_, _) => "LOOP_Jw",
            Self::LSL_Rd_Mw(_, _) => "LSL_Rd_Mw",
            Self::LSL_Rd_Rw(_, _) => "LSL_Rd_Rw",
            Self::LSL_Rw_Mw(_, _) => "LSL_Rw_Mw",
            Self::LSL_Rw_Rw(_, _) => "LSL_Rw_Rw",
            Self::LSS_Rd_Md(_, _) => "LSS_Rd_Md",
            Self::LSS_Rw_Mw(_, _) => "LSS_Rw_Mw",
            Self::LTR_Mw(_) => "LTR_Mw",
            Self::LTR_Rw(_) => "LTR_Rw",
            Self::MOVSB(_) => "MOVSB",
            Self::MOVSD(_) => "MOVSD",
            Self::MOVSW(_) => "MOVSW",
            Self::MOVSX_Rd_Mb(_, _) => "MOVSX_Rd_Mb",
            Self::MOVSX_Rd_Mw(_, _) => "MOVSX_Rd_Mw",
            Self::MOVSX_Rd_Rb(_, _) => "MOVSX_Rd_Rb",
            Self::MOVSX_Rd_Rw(_, _) => "MOVSX_Rd_Rw",
            Self::MOVSX_Rw_Mb(_, _) => "MOVSX_Rw_Mb",
            Self::MOVSX_Rw_Rb(_, _) => "MOVSX_Rw_Rb",
            Self::MOVZX_Rd_Mb(_, _) => "MOVZX_Rd_Mb",
            Self::MOVZX_Rd_Mw(_, _) => "MOVZX_Rd_Mw",
            Self::MOVZX_Rd_Rb(_, _) => "MOVZX_Rd_Rb",
            Self::MOVZX_Rd_Rw(_, _) => "MOVZX_Rd_Rw",
            Self::MOVZX_Rw_Mb(_, _) => "MOVZX_Rw_Mb",
            Self::MOVZX_Rw_Rb(_, _) => "MOVZX_Rw_Rb",
            Self::MOV_Cr_Rd(_, _) => "MOV_Cr_Rd",
            Self::MOV_Dr_Rd(_, _) => "MOV_Dr_Rd",
            Self::MOV_Mb_Ib(_, _) => "MOV_Mb_Ib",
            Self::MOV_Mb_Rb(_, _) => "MOV_Mb_Rb",
            Self::MOV_Md_Id(_, _) => "MOV_Md_Id",
            Self::MOV_Md_Rd(_, _) => "MOV_Md_Rd",
            Self::MOV_Md_Sr(_, _) => "MOV_Md_Sr",
            Self::MOV_Mw_Iw(_, _) => "MOV_Mw_Iw",
            Self::MOV_Mw_Rw(_, _) => "MOV_Mw_Rw",
            Self::MOV_Mw_Sr(_, _) => "MOV_Mw_Sr",
            Self::MOV_Rb_Ib(_, _) => "MOV_Rb_Ib",
            Self::MOV_Rb_Mb(_, _) => "MOV_Rb_Mb",
            Self::MOV_Rb_Rb(_, _) => "MOV_Rb_Rb",
            Self::MOV_Rd_Cr(_, _) => "MOV_Rd_Cr",
            Self::MOV_Rd_Dr(_, _) => "MOV_Rd_Dr",
            Self::MOV_Rd_Id(_, _) => "MOV_Rd_Id",
            Self::MOV_Rd_Md(_, _) => "MOV_Rd_Md",
            Self::MOV_Rd_Rd(_, _) => "MOV_Rd_Rd",
            Self::MOV_Rd_Sr(_, _) => "MOV_Rd_Sr",
            Self::MOV_Rw_Iw(_, _) => "MOV_Rw_Iw",
            Self::MOV_Rw_Mw(_, _) => "MOV_Rw_Mw",
            Self::MOV_Rw_Rw(_, _) => "MOV_Rw_Rw",
            Self::MOV_Rw_Sr(_, _) => "MOV_Rw_Sr",
            Self::MOV_Sr_Md(_, _) => "MOV_Sr_Md",
            Self::MOV_Sr_Mw(_, _) => "MOV_Sr_Mw",
            Self::MOV_Sr_Rd(_, _) => "MOV_Sr_Rd",
            Self::MOV_Sr_Rw(_, _) => "MOV_Sr_Rw",
            Self::MUL_Mb(_) => "MUL_Mb",
            Self::MUL_Md(_) => "MUL_Md",
            Self::MUL_Mw(_) => "MUL_Mw",
            Self::MUL_Rb(_) => "MUL_Rb",
            Self::MUL_Rd(_) => "MUL_Rd",
            Self::MUL_Rw(_) => "MUL_Rw",
            Self::NEG_Mb(_) => "NEG_Mb",
            Self::NEG_Md(_) => "NEG_Md",
            Self::NEG_Mw(_) => "NEG_Mw",
            Self::NEG_Rb(_) => "NEG_Rb",
            Self::NEG_Rd(_) => "NEG_Rd",
            Self::NEG_Rw(_) => "NEG_Rw",
            Self::NOP => "NOP",
            Self::NOT_Mb(_) => "NOT_Mb",
            Self::NOT_Md(_) => "NOT_Md",
            Self::NOT_Mw(_) => "NOT_Mw",
            Self::NOT_Rb(_) => "NOT_Rb",
            Self::NOT_Rd(_) => "NOT_Rd",
            Self::NOT_Rw(_) => "NOT_Rw",
            Self::OR_Mb_Ib(_, _) => "OR_Mb_Ib",
            Self::OR_Mb_Rb(_, _) => "OR_Mb_Rb",
            Self::OR_Md_Id(_, _) => "OR_Md_Id",
            Self::OR_Md_Rd(_, _) => "OR_Md_Rd",
            Self::OR_Mw_Iw(_, _) => "OR_Mw_Iw",
            Self::OR_Mw_Rw(_, _) => "OR_Mw_Rw",
            Self::OR_Rb_Ib(_, _) => "OR_Rb_Ib",
            Self::OR_Rb_Mb(_, _) => "OR_Rb_Mb",
            Self::OR_Rb_Rb(_, _) => "OR_Rb_Rb",
            Self::OR_Rd_Id(_, _) => "OR_Rd_Id",
            Self::OR_Rd_Md(_, _) => "OR_Rd_Md",
            Self::OR_Rd_Rd(_, _) => "OR_Rd_Rd",
            Self::OR_Rw_Iw(_, _) => "OR_Rw_Iw",
            Self::OR_Rw_Mw(_, _) => "OR_Rw_Mw",
            Self::OR_Rw_Rw(_, _) => "OR_Rw_Rw",
            Self::OUTSB(_) => "OUTSB",
            Self::OUTSD(_) => "OUTSD",
            Self::OUTSW(_) => "OUTSW",
            Self::OUT_Dx_Ad => "OUT_Dx_Ad",
            Self::OUT_Dx_Al => "OUT_Dx_Al",
            Self::OUT_Dx_Aw => "OUT_Dx_Aw",
            Self::OUT_Ib_Ad(_) => "OUT_Ib_Ad",
            Self::OUT_Ib_Al(_) => "OUT_Ib_Al",
            Self::OUT_Ib_Aw(_) => "OUT_Ib_Aw",
            Self::PAUSE => "PAUSE",
            Self::POPA => "POPA",
            Self::POPAD => "POPAD",
            Self::POPCNT_Rd_Md(_, _) => "POPCNT_Rd_Md",
            Self::POPCNT_Rd_Rd(_, _) => "POPCNT_Rd_Rd",
            Self::POPCNT_Rw_Mw(_, _) => "POPCNT_Rw_Mw",
            Self::POPCNT_Rw_Rw(_, _) => "POPCNT_Rw_Rw",
            Self::POPF => "POPF",
            Self::POPFD => "POPFD",
            Self::POP_Md(_) => "POP_Md",
            Self::POP_Mw(_) => "POP_Mw",
            Self::POP_Rd(_) => "POP_Rd",
            Self::POP_Rw(_) => "POP_Rw",
            Self::POP_SrO16(_) => "POP_SrO16",
            Self::POP_SrO32(_) => "POP_SrO32",
            Self::PUSHA => "PUSHA",
            Self::PUSHAD => "PUSHAD",
            Self::PUSHF => "PUSHF",
            Self::PUSHFD => "PUSHFD",
            Self::PUSH_Id(_) => "PUSH_Id",
            Self::PUSH_Iw(_) => "PUSH_Iw",
            Self::PUSH_Md(_) => "PUSH_Md",
            Self::PUSH_Mw(_) => "PUSH_Mw",
            Self::PUSH_Rd(_) => "PUSH_Rd",
            Self::PUSH_Rw(_) => "PUSH_Rw",
            Self::PUSH_SrO16(_) => "PUSH_SrO16",
            Self::PUSH_SrO32(_) => "PUSH_SrO32",
            Self::RCL_Mb_Cl(_) => "RCL_Mb_Cl",
            Self::RCL_Mb_Ib(_, _) => "RCL_Mb_Ib",
            Self::RCL_Md_Cl(_) => "RCL_Md_Cl",
            Self::RCL_Md_Ib(_, _) => "RCL_Md_Ib",
            Self::RCL_Mw_Cl(_) => "RCL_Mw_Cl",
            Self::RCL_Mw_Ib(_, _) => "RCL_Mw_Ib",
            Self::RCL_Rb_Cl(_) => "RCL_Rb_Cl",
            Self::RCL_Rb_Ib(_, _) => "RCL_Rb_Ib",
            Self::RCL_Rd_Cl(_) => "RCL_Rd_Cl",
            Self::RCL_Rd_Ib(_, _) => "RCL_Rd_Ib",
            Self::RCL_Rw_Cl(_) => "RCL_Rw_Cl",
            Self::RCL_Rw_Ib(_, _) => "RCL_Rw_Ib",
            Self::RCR_Mb_Cl(_) => "RCR_Mb_Cl",
            Self::RCR_Mb_Ib(_, _) => "RCR_Mb_Ib",
            Self::RCR_Md_Cl(_) => "RCR_Md_Cl",
            Self::RCR_Md_Ib(_, _) => "RCR_Md_Ib",
            Self::RCR_Mw_Cl(_) => "RCR_Mw_Cl",
            Self::RCR_Mw_Ib(_, _) => "RCR_Mw_Ib",
            Self::RCR_Rb_Cl(_) => "RCR_Rb_Cl",
            Self::RCR_Rb_Ib(_, _) => "RCR_Rb_Ib",
            Self::RCR_Rd_Cl(_) => "RCR_Rd_Cl",
            Self::RCR_Rd_Ib(_, _) => "RCR_Rd_Ib",
            Self::RCR_Rw_Cl(_) => "RCR_Rw_Cl",
            Self::RCR_Rw_Ib(_, _) => "RCR_Rw_Ib",
            Self::RDMSR => "RDMSR",
            Self::RDTSC => "RDTSC",
            Self::RDTSCP => "RDTSCP",
            Self::REPNZ_CMPSB(_) => "REPNZ_CMPSB",
            Self::REPNZ_CMPSD(_) => "REPNZ_CMPSD",
            Self::REPNZ_CMPSW(_) => "REPNZ_CMPSW",
            Self::REPNZ_SCASB(_) => "REPNZ_SCASB",
            Self::REPNZ_SCASD(_) => "REPNZ_SCASD",
            Self::REPNZ_SCASW(_) => "REPNZ_SCASW",
            Self::REPZ_CMPSB(_) => "REPZ_CMPSB",
            Self::REPZ_CMPSD(_) => "REPZ_CMPSD",
            Self::REPZ_CMPSW(_) => "REPZ_CMPSW",
            Self::REPZ_SCASB(_) => "REPZ_SCASB",
            Self::REPZ_SCASD(_) => "REPZ_SCASD",
            Self::REPZ_SCASW(_) => "REPZ_SCASW",
            Self::REP_INSB(_) => "REP_INSB",
            Self::REP_INSD(_) => "REP_INSD",
            Self::REP_INSW(_) => "REP_INSW",
            Self::REP_LODSB(_) => "REP_LODSB",
            Self::REP_LODSD(_) => "REP_LODSD",
            Self::REP_LODSW(_) => "REP_LODSW",
            Self::REP_MOVSB(_) => "REP_MOVSB",
            Self::REP_MOVSD(_) => "REP_MOVSD",
            Self::REP_MOVSW(_) => "REP_MOVSW",
            Self::REP_OUTSB(_) => "REP_OUTSB",
            Self::REP_OUTSD(_) => "REP_OUTSD",
            Self::REP_OUTSW(_) => "REP_OUTSW",
            Self::REP_STOSB(_) => "REP_STOSB",
            Self::REP_STOSD(_) => "REP_STOSD",
            Self::REP_STOSW(_) => "REP_STOSW",
            Self::RETF_D16(_) => "RETF_D16",
            Self::RETF_D32(_) => "RETF_D32",
            Self::RET_D16(_) => "RET_D16",
            Self::RET_D32(_) => "RET_D32",
            Self::ROL_Mb_Cl(_) => "ROL_Mb_Cl",
            Self::ROL_Mb_Ib(_, _) => "ROL_Mb_Ib",
            Self::ROL_Md_Cl(_) => "ROL_Md_Cl",
            Self::ROL_Md_Ib(_, _) => "ROL_Md_Ib",
            Self::ROL_Mw_Cl(_) => "ROL_Mw_Cl",
            Self::ROL_Mw_Ib(_, _) => "ROL_Mw_Ib",
            Self::ROL_Rb_Cl(_) => "ROL_Rb_Cl",
            Self::ROL_Rb_Ib(_, _) => "ROL_Rb_Ib",
            Self::ROL_Rd_Cl(_) => "ROL_Rd_Cl",
            Self::ROL_Rd_Ib(_, _) => "ROL_Rd_Ib",
            Self::ROL_Rw_Cl(_) => "ROL_Rw_Cl",
            Self::ROL_Rw_Ib(_, _) => "ROL_Rw_Ib",
            Self::ROR_Mb_Cl(_) => "ROR_Mb_Cl",
            Self::ROR_Mb_Ib(_, _) => "ROR_Mb_Ib",
            Self::ROR_Md_Cl(_) => "ROR_Md_Cl",
            Self::ROR_Md_Ib(_, _) => "ROR_Md_Ib",
            Self::ROR_Mw_Cl(_) => "ROR_Mw_Cl",
            Self::ROR_Mw_Ib(_, _) => "ROR_Mw_Ib",
            Self::ROR_Rb_Cl(_) => "ROR_Rb_Cl",
            Self::ROR_Rb_Ib(_, _) => "ROR_Rb_Ib",
            Self::ROR_Rd_Cl(_) => "ROR_Rd_Cl",
            Self::ROR_Rd_Ib(_, _) => "ROR_Rd_Ib",
            Self::ROR_Rw_Cl(_) => "ROR_Rw_Cl",
            Self::ROR_Rw_Ib(_, _) => "ROR_Rw_Ib",
            Self::RSM => "RSM",
            Self::SAHF => "SAHF",
            Self::SALC => "SALC",
            Self::SAR_Mb_Cl(_) => "SAR_Mb_Cl",
            Self::SAR_Mb_Ib(_, _) => "SAR_Mb_Ib",
            Self::SAR_Md_Cl(_) => "SAR_Md_Cl",
            Self::SAR_Md_Ib(_, _) => "SAR_Md_Ib",
            Self::SAR_Mw_Cl(_) => "SAR_Mw_Cl",
            Self::SAR_Mw_Ib(_, _) => "SAR_Mw_Ib",
            Self::SAR_Rb_Cl(_) => "SAR_Rb_Cl",
            Self::SAR_Rb_Ib(_, _) => "SAR_Rb_Ib",
            Self::SAR_Rd_Cl(_) => "SAR_Rd_Cl",
            Self::SAR_Rd_Ib(_, _) => "SAR_Rd_Ib",
            Self::SAR_Rw_Cl(_) => "SAR_Rw_Cl",
            Self::SAR_Rw_Ib(_, _) => "SAR_Rw_Ib",
            Self::SBB_Mb_Ib(_, _) => "SBB_Mb_Ib",
            Self::SBB_Mb_Rb(_, _) => "SBB_Mb_Rb",
            Self::SBB_Md_Id(_, _) => "SBB_Md_Id",
            Self::SBB_Md_Rd(_, _) => "SBB_Md_Rd",
            Self::SBB_Mw_Iw(_, _) => "SBB_Mw_Iw",
            Self::SBB_Mw_Rw(_, _) => "SBB_Mw_Rw",
            Self::SBB_Rb_Ib(_, _) => "SBB_Rb_Ib",
            Self::SBB_Rb_Mb(_, _) => "SBB_Rb_Mb",
            Self::SBB_Rb_Rb(_, _) => "SBB_Rb_Rb",
            Self::SBB_Rd_Id(_, _) => "SBB_Rd_Id",
            Self::SBB_Rd_Md(_, _) => "SBB_Rd_Md",
            Self::SBB_Rd_Rd(_, _) => "SBB_Rd_Rd",
            Self::SBB_Rw_Iw(_, _) => "SBB_Rw_Iw",
            Self::SBB_Rw_Mw(_, _) => "SBB_Rw_Mw",
            Self::SBB_Rw_Rw(_, _) => "SBB_Rw_Rw",
            Self::SCASB(_) => "SCASB",
            Self::SCASD(_) => "SCASD",
            Self::SCASW(_) => "SCASW",
            Self::SETCC_Mb(_, _) => "SETCC_Mb",
            Self::SETCC_Rb(_, _) => "SETCC_Rb",
            Self::SGDT_Mp(_) => "SGDT_Mp",
            Self::SHLD_Md_Rd_Cl(_, _) => "SHLD_Md_Rd_Cl",
            Self::SHLD_Md_Rd_Ib(_, _, _) => "SHLD_Md_Rd_Ib",
            Self::SHLD_Mw_Rw_Cl(_, _) => "SHLD_Mw_Rw_Cl",
            Self::SHLD_Mw_Rw_Ib(_, _, _) => "SHLD_Mw_Rw_Ib",
            Self::SHLD_Rd_Rd_Cl(_, _) => "SHLD_Rd_Rd_Cl",
            Self::SHLD_Rd_Rd_Ib(_, _, _) => "SHLD_Rd_Rd_Ib",
            Self::SHLD_Rw_Rw_Cl(_, _) => "SHLD_Rw_Rw_Cl",
            Self::SHLD_Rw_Rw_Ib(_, _, _) => "SHLD_Rw_Rw_Ib",
            Self::SHL_Mb_Cl(_) => "SHL_Mb_Cl",
            Self::SHL_Mb_Ib(_, _) => "SHL_Mb_Ib",
            Self::SHL_Md_Cl(_) => "SHL_Md_Cl",
            Self::SHL_Md_Ib(_, _) => "SHL_Md_Ib",
            Self::SHL_Mw_Cl(_) => "SHL_Mw_Cl",
            Self::SHL_Mw_Ib(_, _) => "SHL_Mw_Ib",
            Self::SHL_Rb_Cl(_) => "SHL_Rb_Cl",
            Self::SHL_Rb_Ib(_, _) => "SHL_Rb_Ib",
            Self::SHL_Rd_Cl(_) => "SHL_Rd_Cl",
            Self::SHL_Rd_Ib(_, _) => "SHL_Rd_Ib",
            Self::SHL_Rw_Cl(_) => "SHL_Rw_Cl",
            Self::SHL_Rw_Ib(_, _) => "SHL_Rw_Ib",
            Self::SHRD_Md_Rd_Cl(_, _) => "SHRD_Md_Rd_Cl",
            Self::SHRD_Md_Rd_Ib(_, _, _) => "SHRD_Md_Rd_Ib",
            Self::SHRD_Mw_Rw_Cl(_, _) => "SHRD_Mw_Rw_Cl",
            Self::SHRD_Mw_Rw_Ib(_, _, _) => "SHRD_Mw_Rw_Ib",
            Self::SHRD_Rd_Rd_Cl(_, _) => "SHRD_Rd_Rd_Cl",
            Self::SHRD_Rd_Rd_Ib(_, _, _) => "SHRD_Rd_Rd_Ib",
            Self::SHRD_Rw_Rw_Cl(_, _) => "SHRD_Rw_Rw_Cl",
            Self::SHRD_Rw_Rw_Ib(_, _, _) => "SHRD_Rw_Rw_Ib",
            Self::SHR_Mb_Cl(_) => "SHR_Mb_Cl",
            Self::SHR_Mb_Ib(_, _) => "SHR_Mb_Ib",
            Self::SHR_Md_Cl(_) => "SHR_Md_Cl",
            Self::SHR_Md_Ib(_, _) => "SHR_Md_Ib",
            Self::SHR_Mw_Cl(_) => "SHR_Mw_Cl",
            Self::SHR_Mw_Ib(_, _) => "SHR_Mw_Ib",
            Self::SHR_Rb_Cl(_) => "SHR_Rb_Cl",
            Self::SHR_Rb_Ib(_, _) => "SHR_Rb_Ib",
            Self::SHR_Rd_Cl(_) => "SHR_Rd_Cl",
            Self::SHR_Rd_Ib(_, _) => "SHR_Rd_Ib",
            Self::SHR_Rw_Cl(_) => "SHR_Rw_Cl",
            Self::SHR_Rw_Ib(_, _) => "SHR_Rw_Ib",
            Self::SIDT_Mp(_) => "SIDT_Mp",
            Self::SIMD(_) => "SIMD",
            Self::SIMD_M(_, _, _, _) => "SIMD_M",
            Self::SIMD_R(_, _, _, _) => "SIMD_R",
            Self::SIMD_S(_, _, _, _) => "SIMD_S",
            Self::SLDT_Md(_) => "SLDT_Md",
            Self::SLDT_Mw(_) => "SLDT_Mw",
            Self::SLDT_Rd(_) => "SLDT_Rd",
            Self::SLDT_Rw(_) => "SLDT_Rw",
            Self::SMSW_Mw(_) => "SMSW_Mw",
            Self::SMSW_Rw(_) => "SMSW_Rw",
            Self::STC => "STC",
            Self::STD => "STD",
            Self::STI => "STI",
            Self::STOSB(_) => "STOSB",
            Self::STOSD(_) => "STOSD",
            Self::STOSW(_) => "STOSW",
            Self::STR_Md(_) => "STR_Md",
            Self::STR_Mw(_) => "STR_Mw",
            Self::STR_Rd(_) => "STR_Rd",
            Self::STR_Rw(_) => "STR_Rw",
            Self::SUB_Mb_Ib(_, _) => "SUB_Mb_Ib",
            Self::SUB_Mb_Rb(_, _) => "SUB_Mb_Rb",
            Self::SUB_Md_Id(_, _) => "SUB_Md_Id",
            Self::SUB_Md_Rd(_, _) => "SUB_Md_Rd",
            Self::SUB_Mw_Iw(_, _) => "SUB_Mw_Iw",
            Self::SUB_Mw_Rw(_, _) => "SUB_Mw_Rw",
            Self::SUB_Rb_Ib(_, _) => "SUB_Rb_Ib",
            Self::SUB_Rb_Mb(_, _) => "SUB_Rb_Mb",
            Self::SUB_Rb_Rb(_, _) => "SUB_Rb_Rb",
            Self::SUB_Rd_Id(_, _) => "SUB_Rd_Id",
            Self::SUB_Rd_Md(_, _) => "SUB_Rd_Md",
            Self::SUB_Rd_Rd(_, _) => "SUB_Rd_Rd",
            Self::SUB_Rw_Iw(_, _) => "SUB_Rw_Iw",
            Self::SUB_Rw_Mw(_, _) => "SUB_Rw_Mw",
            Self::SUB_Rw_Rw(_, _) => "SUB_Rw_Rw",
            Self::SWAPGS => "SWAPGS",
            Self::SYSCALL => "SYSCALL",
            Self::SYSENTER => "SYSENTER",
            Self::SYSEXIT => "SYSEXIT",
            Self::SYSRET => "SYSRET",
            Self::TEST_Mb_Ib(_, _) => "TEST_Mb_Ib",
            Self::TEST_Mb_Rb(_, _) => "TEST_Mb_Rb",
            Self::TEST_Md_Id(_, _) => "TEST_Md_Id",
            Self::TEST_Md_Rd(_, _) => "TEST_Md_Rd",
            Self::TEST_Mw_Iw(_, _) => "TEST_Mw_Iw",
            Self::TEST_Mw_Rw(_, _) => "TEST_Mw_Rw",
            Self::TEST_Rb_Ib(_, _) => "TEST_Rb_Ib",
            Self::TEST_Rb_Rb(_, _) => "TEST_Rb_Rb",
            Self::TEST_Rd_Id(_, _) => "TEST_Rd_Id",
            Self::TEST_Rd_Rd(_, _) => "TEST_Rd_Rd",
            Self::TEST_Rw_Iw(_, _) => "TEST_Rw_Iw",
            Self::TEST_Rw_Rw(_, _) => "TEST_Rw_Rw",
            Self::UD => "UD",
            Self::VERR_Mw(_) => "VERR_Mw",
            Self::VERR_Rw(_) => "VERR_Rw",
            Self::VERW_Mw(_) => "VERW_Mw",
            Self::VERW_Rw(_) => "VERW_Rw",
            Self::WAIT => "WAIT",
            Self::WBINVD => "WBINVD",
            Self::WRMSR => "WRMSR",
            Self::XADD_Mb_Rb(_, _) => "XADD_Mb_Rb",
            Self::XADD_Md_Rd(_, _) => "XADD_Md_Rd",
            Self::XADD_Mw_Rw(_, _) => "XADD_Mw_Rw",
            Self::XADD_Rb_Rb(_, _) => "XADD_Rb_Rb",
            Self::XADD_Rd_Rd(_, _) => "XADD_Rd_Rd",
            Self::XADD_Rw_Rw(_, _) => "XADD_Rw_Rw",
            Self::XCHG_Mb_Rb(_, _) => "XCHG_Mb_Rb",
            Self::XCHG_Md_Rd(_, _) => "XCHG_Md_Rd",
            Self::XCHG_Mw_Rw(_, _) => "XCHG_Mw_Rw",
            Self::XCHG_Rb_Rb(_, _) => "XCHG_Rb_Rb",
            Self::XCHG_Rd_Rd(_, _) => "XCHG_Rd_Rd",
            Self::XCHG_Rw_Rw(_, _) => "XCHG_Rw_Rw",
            Self::XGETBV => "XGETBV",
            Self::XLAT_Mb(_) => "XLAT_Mb",
            Self::XOR_Mb_Ib(_, _) => "XOR_Mb_Ib",
            Self::XOR_Mb_Rb(_, _) => "XOR_Mb_Rb",
            Self::XOR_Md_Id(_, _) => "XOR_Md_Id",
            Self::XOR_Md_Rd(_, _) => "XOR_Md_Rd",
            Self::XOR_Mw_Iw(_, _) => "XOR_Mw_Iw",
            Self::XOR_Mw_Rw(_, _) => "XOR_Mw_Rw",
            Self::XOR_Rb_Ib(_, _) => "XOR_Rb_Ib",
            Self::XOR_Rb_Mb(_, _) => "XOR_Rb_Mb",
            Self::XOR_Rb_Rb(_, _) => "XOR_Rb_Rb",
            Self::XOR_Rd_Id(_, _) => "XOR_Rd_Id",
            Self::XOR_Rd_Md(_, _) => "XOR_Rd_Md",
            Self::XOR_Rd_Rd(_, _) => "XOR_Rd_Rd",
            Self::XOR_Rw_Iw(_, _) => "XOR_Rw_Iw",
            Self::XOR_Rw_Mw(_, _) => "XOR_Rw_Mw",
            Self::XOR_Rw_Rw(_, _) => "XOR_Rw_Rw",
            Self::XSETBV => "XSETBV",
        }
    }

    pub fn n_args(&self) -> usize {
        match self {
            Self::AAA => 0,
            Self::AAD_Ib(_) => 1,
            Self::AAM_Ib(_) => 1,
            Self::AAS => 0,
            Self::ADC_Mb_Ib(_, _) => 2,
            Self::ADC_Mb_Rb(_, _) => 2,
            Self::ADC_Md_Id(_, _) => 2,
            Self::ADC_Md_Rd(_, _) => 2,
            Self::ADC_Mw_Iw(_, _) => 2,
            Self::ADC_Mw_Rw(_, _) => 2,
            Self::ADC_Rb_Ib(_, _) => 2,
            Self::ADC_Rb_Mb(_, _) => 2,
            Self::ADC_Rb_Rb(_, _) => 2,
            Self::ADC_Rd_Id(_, _) => 2,
            Self::ADC_Rd_Md(_, _) => 2,
            Self::ADC_Rd_Rd(_, _) => 2,
            Self::ADC_Rw_Iw(_, _) => 2,
            Self::ADC_Rw_Mw(_, _) => 2,
            Self::ADC_Rw_Rw(_, _) => 2,
            Self::ADD_Mb_Ib(_, _) => 2,
            Self::ADD_Mb_Rb(_, _) => 2,
            Self::ADD_Md_Id(_, _) => 2,
            Self::ADD_Md_Rd(_, _) => 2,
            Self::ADD_Mw_Iw(_, _) => 2,
            Self::ADD_Mw_Rw(_, _) => 2,
            Self::ADD_Rb_Ib(_, _) => 2,
            Self::ADD_Rb_Mb(_, _) => 2,
            Self::ADD_Rb_Rb(_, _) => 2,
            Self::ADD_Rd_Id(_, _) => 2,
            Self::ADD_Rd_Md(_, _) => 2,
            Self::ADD_Rd_Rd(_, _) => 2,
            Self::ADD_Rw_Iw(_, _) => 2,
            Self::ADD_Rw_Mw(_, _) => 2,
            Self::ADD_Rw_Rw(_, _) => 2,
            Self::AND_Mb_Ib(_, _) => 2,
            Self::AND_Mb_Rb(_, _) => 2,
            Self::AND_Md_Id(_, _) => 2,
            Self::AND_Md_Rd(_, _) => 2,
            Self::AND_Mw_Iw(_, _) => 2,
            Self::AND_Mw_Rw(_, _) => 2,
            Self::AND_Rb_Ib(_, _) => 2,
            Self::AND_Rb_Mb(_, _) => 2,
            Self::AND_Rb_Rb(_, _) => 2,
            Self::AND_Rd_Id(_, _) => 2,
            Self::AND_Rd_Md(_, _) => 2,
            Self::AND_Rd_Rd(_, _) => 2,
            Self::AND_Rw_Iw(_, _) => 2,
            Self::AND_Rw_Mw(_, _) => 2,
            Self::AND_Rw_Rw(_, _) => 2,
            Self::ARPL_Mw_Rw(_, _) => 2,
            Self::ARPL_Rw_Rw(_, _) => 2,
            Self::BOUND_Rd_Md(_, _) => 2,
            Self::BOUND_Rw_Mw(_, _) => 2,
            Self::BSF_Rd_Md(_, _) => 2,
            Self::BSF_Rd_Rd(_, _) => 2,
            Self::BSF_Rw_Mw(_, _) => 2,
            Self::BSF_Rw_Rw(_, _) => 2,
            Self::BSR_Rd_Md(_, _) => 2,
            Self::BSR_Rd_Rd(_, _) => 2,
            Self::BSR_Rw_Mw(_, _) => 2,
            Self::BSR_Rw_Rw(_, _) => 2,
            Self::BSWAP_Rd(_) => 1,
            Self::BTC_Md_Ib(_, _) => 2,
            Self::BTC_Md_Rd(_, _) => 2,
            Self::BTC_Mw_Ib(_, _) => 2,
            Self::BTC_Mw_Rw(_, _) => 2,
            Self::BTC_Rd_Ib(_, _) => 2,
            Self::BTC_Rd_Rd(_, _) => 2,
            Self::BTC_Rw_Ib(_, _) => 2,
            Self::BTC_Rw_Rw(_, _) => 2,
            Self::BTR_Md_Ib(_, _) => 2,
            Self::BTR_Md_Rd(_, _) => 2,
            Self::BTR_Mw_Ib(_, _) => 2,
            Self::BTR_Mw_Rw(_, _) => 2,
            Self::BTR_Rd_Ib(_, _) => 2,
            Self::BTR_Rd_Rd(_, _) => 2,
            Self::BTR_Rw_Ib(_, _) => 2,
            Self::BTR_Rw_Rw(_, _) => 2,
            Self::BTS_Md_Ib(_, _) => 2,
            Self::BTS_Md_Rd(_, _) => 2,
            Self::BTS_Mw_Ib(_, _) => 2,
            Self::BTS_Mw_Rw(_, _) => 2,
            Self::BTS_Rd_Ib(_, _) => 2,
            Self::BTS_Rd_Rd(_, _) => 2,
            Self::BTS_Rw_Ib(_, _) => 2,
            Self::BTS_Rw_Rw(_, _) => 2,
            Self::BT_Md_Ib(_, _) => 2,
            Self::BT_Md_Rd(_, _) => 2,
            Self::BT_Mw_Ib(_, _) => 2,
            Self::BT_Mw_Rw(_, _) => 2,
            Self::BT_Rd_Ib(_, _) => 2,
            Self::BT_Rd_Rd(_, _) => 2,
            Self::BT_Rw_Ib(_, _) => 2,
            Self::BT_Rw_Rw(_, _) => 2,
            Self::CALLF_ApO16(_, _) => 2,
            Self::CALLF_ApO32(_, _) => 2,
            Self::CALLF_MpO16(_) => 1,
            Self::CALLF_MpO32(_) => 1,
            Self::CALL_Jd(_) => 1,
            Self::CALL_Jw(_) => 1,
            Self::CALL_Md(_) => 1,
            Self::CALL_Mw(_) => 1,
            Self::CALL_Rd(_) => 1,
            Self::CALL_Rw(_) => 1,
            Self::CDQ => 0,
            Self::CLC => 0,
            Self::CLD => 0,
            Self::CLI => 0,
            Self::CLTS => 0,
            Self::CMC => 0,
            Self::CMOV_Rd_Md(_, _, _) => 3,
            Self::CMOV_Rd_Rd(_, _, _) => 3,
            Self::CMOV_Rw_Mw(_, _, _) => 3,
            Self::CMOV_Rw_Rw(_, _, _) => 3,
            Self::CMPSB(_) => 1,
            Self::CMPSD(_) => 1,
            Self::CMPSW(_) => 1,
            Self::CMPXCHG_Mb_Rb(_, _) => 2,
            Self::CMPXCHG_Md_Rd(_, _) => 2,
            Self::CMPXCHG_Mw_Rw(_, _) => 2,
            Self::CMPXCHG_Rb_Rb(_, _) => 2,
            Self::CMPXCHG_Rd_Rd(_, _) => 2,
            Self::CMPXCHG_Rw_Rw(_, _) => 2,
            Self::CMP_Mb_Ib(_, _) => 2,
            Self::CMP_Mb_Rb(_, _) => 2,
            Self::CMP_Md_Id(_, _) => 2,
            Self::CMP_Md_Rd(_, _) => 2,
            Self::CMP_Mw_Iw(_, _) => 2,
            Self::CMP_Mw_Rw(_, _) => 2,
            Self::CMP_Rb_Ib(_, _) => 2,
            Self::CMP_Rb_Mb(_, _) => 2,
            Self::CMP_Rb_Rb(_, _) => 2,
            Self::CMP_Rd_Id(_, _) => 2,
            Self::CMP_Rd_Md(_, _) => 2,
            Self::CMP_Rd_Rd(_, _) => 2,
            Self::CMP_Rw_Iw(_, _) => 2,
            Self::CMP_Rw_Mw(_, _) => 2,
            Self::CMP_Rw_Rw(_, _) => 2,
            Self::CPUID => 0,
            Self::CWD => 0,
            Self::DAA => 0,
            Self::DAS => 0,
            Self::DEC_Mb(_) => 1,
            Self::DEC_Md(_) => 1,
            Self::DEC_Mw(_) => 1,
            Self::DEC_Rb(_) => 1,
            Self::DEC_Rd(_) => 1,
            Self::DEC_Rw(_) => 1,
            Self::DIV_Mb(_) => 1,
            Self::DIV_Md(_) => 1,
            Self::DIV_Mw(_) => 1,
            Self::DIV_Rb(_) => 1,
            Self::DIV_Rd(_) => 1,
            Self::DIV_Rw(_) => 1,
            Self::ENTER_Iw_Ib(_, _) => 2,
            Self::ESC_Mb(_, _, _) => 3,
            Self::ESC_Rb(_, _, _) => 3,
            Self::HLT => 0,
            Self::ICEBP => 0,
            Self::IDIV_Mb(_) => 1,
            Self::IDIV_Md(_) => 1,
            Self::IDIV_Mw(_) => 1,
            Self::IDIV_Rb(_) => 1,
            Self::IDIV_Rd(_) => 1,
            Self::IDIV_Rw(_) => 1,
            Self::IMUL_Mb(_) => 1,
            Self::IMUL_Md(_) => 1,
            Self::IMUL_Mw(_) => 1,
            Self::IMUL_Rb(_) => 1,
            Self::IMUL_Rd(_) => 1,
            Self::IMUL_Rd_Md(_, _) => 2,
            Self::IMUL_Rd_Md_Id(_, _, _) => 3,
            Self::IMUL_Rd_Rd(_, _) => 2,
            Self::IMUL_Rd_Rd_Id(_, _, _) => 3,
            Self::IMUL_Rw(_) => 1,
            Self::IMUL_Rw_Mw(_, _) => 2,
            Self::IMUL_Rw_Mw_Iw(_, _, _) => 3,
            Self::IMUL_Rw_Rw(_, _) => 2,
            Self::IMUL_Rw_Rw_Iw(_, _, _) => 3,
            Self::INC_Mb(_) => 1,
            Self::INC_Md(_) => 1,
            Self::INC_Mw(_) => 1,
            Self::INC_Rb(_) => 1,
            Self::INC_Rd(_) => 1,
            Self::INC_Rw(_) => 1,
            Self::INSB(_) => 1,
            Self::INSD(_) => 1,
            Self::INSW(_) => 1,
            Self::INT3 => 0,
            Self::INTO => 0,
            Self::INT_Ib(_) => 1,
            Self::INVD => 0,
            Self::INVLPG_Mb(_) => 1,
            Self::INVLPG_Md(_) => 1,
            Self::INVLPG_Mw(_) => 1,
            Self::INVLPG_Rb(_) => 1,
            Self::INVLPG_Rd(_) => 1,
            Self::INVLPG_Rw(_) => 1,
            Self::IN_Ad_Dx => 0,
            Self::IN_Ad_Ib(_) => 1,
            Self::IN_Al_Dx => 0,
            Self::IN_Al_Ib(_) => 1,
            Self::IN_Aw_Dx => 0,
            Self::IN_Aw_Ib(_) => 1,
            Self::IRET => 0,
            Self::IRETD => 0,
            Self::JCC_Jd(_, _) => 2,
            Self::JCC_Jw(_, _) => 2,
            Self::JCXZ_Jd(_) => 1,
            Self::JCXZ_Jw(_) => 1,
            Self::JECXZ_Jd(_) => 1,
            Self::JECXZ_Jw(_) => 1,
            Self::JMPF_ApO16(_, _) => 2,
            Self::JMPF_ApO32(_, _) => 2,
            Self::JMPF_MpO16(_) => 1,
            Self::JMPF_MpO32(_) => 1,
            Self::JMP_Jd(_) => 1,
            Self::JMP_Jw(_) => 1,
            Self::JMP_Md(_) => 1,
            Self::JMP_Mw(_) => 1,
            Self::JMP_Rd(_) => 1,
            Self::JMP_Rw(_) => 1,
            Self::LAHF => 0,
            Self::LAR_Rd_Mw(_, _) => 2,
            Self::LAR_Rd_Rw(_, _) => 2,
            Self::LAR_Rw_Mw(_, _) => 2,
            Self::LAR_Rw_Rw(_, _) => 2,
            Self::LDS_Rd_Md(_, _) => 2,
            Self::LDS_Rw_Mw(_, _) => 2,
            Self::LEAVE => 0,
            Self::LEA_Rd_Md(_, _) => 2,
            Self::LEA_Rw_Mw(_, _) => 2,
            Self::LES_Rd_Md(_, _) => 2,
            Self::LES_Rw_Mw(_, _) => 2,
            Self::LFS_Rd_Md(_, _) => 2,
            Self::LFS_Rw_Mw(_, _) => 2,
            Self::LGDT_MpO16(_) => 1,
            Self::LGDT_MpO32(_) => 1,
            Self::LGS_Rd_Md(_, _) => 2,
            Self::LGS_Rw_Mw(_, _) => 2,
            Self::LIDT_MpO16(_) => 1,
            Self::LIDT_MpO32(_) => 1,
            Self::LLDT_Mw(_) => 1,
            Self::LLDT_Rw(_) => 1,
            Self::LMSW_Mw(_) => 1,
            Self::LMSW_Rw(_) => 1,
            Self::LODSB(_) => 1,
            Self::LODSD(_) => 1,
            Self::LODSW(_) => 1,
            Self::LOOPNZ_Jd(_, _) => 2,
            Self::LOOPNZ_Jw(_, _) => 2,
            Self::LOOPZ_Jd(_, _) => 2,
            Self::LOOPZ_Jw(_, _) => 2,
            Self::LOOP_Jd(_, _) => 2,
            Self::LOOP_Jw(_, _) => 2,
            Self::LSL_Rd_Mw(_, _) => 2,
            Self::LSL_Rd_Rw(_, _) => 2,
            Self::LSL_Rw_Mw(_, _) => 2,
            Self::LSL_Rw_Rw(_, _) => 2,
            Self::LSS_Rd_Md(_, _) => 2,
            Self::LSS_Rw_Mw(_, _) => 2,
            Self::LTR_Mw(_) => 1,
            Self::LTR_Rw(_) => 1,
            Self::MOVSB(_) => 1,
            Self::MOVSD(_) => 1,
            Self::MOVSW(_) => 1,
            Self::MOVSX_Rd_Mb(_, _) => 2,
            Self::MOVSX_Rd_Mw(_, _) => 2,
            Self::MOVSX_Rd_Rb(_, _) => 2,
            Self::MOVSX_Rd_Rw(_, _) => 2,
            Self::MOVSX_Rw_Mb(_, _) => 2,
            Self::MOVSX_Rw_Rb(_, _) => 2,
            Self::MOVZX_Rd_Mb(_, _) => 2,
            Self::MOVZX_Rd_Mw(_, _) => 2,
            Self::MOVZX_Rd_Rb(_, _) => 2,
            Self::MOVZX_Rd_Rw(_, _) => 2,
            Self::MOVZX_Rw_Mb(_, _) => 2,
            Self::MOVZX_Rw_Rb(_, _) => 2,
            Self::MOV_Cr_Rd(_, _) => 2,
            Self::MOV_Dr_Rd(_, _) => 2,
            Self::MOV_Mb_Ib(_, _) => 2,
            Self::MOV_Mb_Rb(_, _) => 2,
            Self::MOV_Md_Id(_, _) => 2,
            Self::MOV_Md_Rd(_, _) => 2,
            Self::MOV_Md_Sr(_, _) => 2,
            Self::MOV_Mw_Iw(_, _) => 2,
            Self::MOV_Mw_Rw(_, _) => 2,
            Self::MOV_Mw_Sr(_, _) => 2,
            Self::MOV_Rb_Ib(_, _) => 2,
            Self::MOV_Rb_Mb(_, _) => 2,
            Self::MOV_Rb_Rb(_, _) => 2,
            Self::MOV_Rd_Cr(_, _) => 2,
            Self::MOV_Rd_Dr(_, _) => 2,
            Self::MOV_Rd_Id(_, _) => 2,
            Self::MOV_Rd_Md(_, _) => 2,
            Self::MOV_Rd_Rd(_, _) => 2,
            Self::MOV_Rd_Sr(_, _) => 2,
            Self::MOV_Rw_Iw(_, _) => 2,
            Self::MOV_Rw_Mw(_, _) => 2,
            Self::MOV_Rw_Rw(_, _) => 2,
            Self::MOV_Rw_Sr(_, _) => 2,
            Self::MOV_Sr_Md(_, _) => 2,
            Self::MOV_Sr_Mw(_, _) => 2,
            Self::MOV_Sr_Rd(_, _) => 2,
            Self::MOV_Sr_Rw(_, _) => 2,
            Self::MUL_Mb(_) => 1,
            Self::MUL_Md(_) => 1,
            Self::MUL_Mw(_) => 1,
            Self::MUL_Rb(_) => 1,
            Self::MUL_Rd(_) => 1,
            Self::MUL_Rw(_) => 1,
            Self::NEG_Mb(_) => 1,
            Self::NEG_Md(_) => 1,
            Self::NEG_Mw(_) => 1,
            Self::NEG_Rb(_) => 1,
            Self::NEG_Rd(_) => 1,
            Self::NEG_Rw(_) => 1,
            Self::NOP => 0,
            Self::NOT_Mb(_) => 1,
            Self::NOT_Md(_) => 1,
            Self::NOT_Mw(_) => 1,
            Self::NOT_Rb(_) => 1,
            Self::NOT_Rd(_) => 1,
            Self::NOT_Rw(_) => 1,
            Self::OR_Mb_Ib(_, _) => 2,
            Self::OR_Mb_Rb(_, _) => 2,
            Self::OR_Md_Id(_, _) => 2,
            Self::OR_Md_Rd(_, _) => 2,
            Self::OR_Mw_Iw(_, _) => 2,
            Self::OR_Mw_Rw(_, _) => 2,
            Self::OR_Rb_Ib(_, _) => 2,
            Self::OR_Rb_Mb(_, _) => 2,
            Self::OR_Rb_Rb(_, _) => 2,
            Self::OR_Rd_Id(_, _) => 2,
            Self::OR_Rd_Md(_, _) => 2,
            Self::OR_Rd_Rd(_, _) => 2,
            Self::OR_Rw_Iw(_, _) => 2,
            Self::OR_Rw_Mw(_, _) => 2,
            Self::OR_Rw_Rw(_, _) => 2,
            Self::OUTSB(_) => 1,
            Self::OUTSD(_) => 1,
            Self::OUTSW(_) => 1,
            Self::OUT_Dx_Ad => 0,
            Self::OUT_Dx_Al => 0,
            Self::OUT_Dx_Aw => 0,
            Self::OUT_Ib_Ad(_) => 1,
            Self::OUT_Ib_Al(_) => 1,
            Self::OUT_Ib_Aw(_) => 1,
            Self::PAUSE => 0,
            Self::POPA => 0,
            Self::POPAD => 0,
            Self::POPCNT_Rd_Md(_, _) => 2,
            Self::POPCNT_Rd_Rd(_, _) => 2,
            Self::POPCNT_Rw_Mw(_, _) => 2,
            Self::POPCNT_Rw_Rw(_, _) => 2,
            Self::POPF => 0,
            Self::POPFD => 0,
            Self::POP_Md(_) => 1,
            Self::POP_Mw(_) => 1,
            Self::POP_Rd(_) => 1,
            Self::POP_Rw(_) => 1,
            Self::POP_SrO16(_) => 1,
            Self::POP_SrO32(_) => 1,
            Self::PUSHA => 0,
            Self::PUSHAD => 0,
            Self::PUSHF => 0,
            Self::PUSHFD => 0,
            Self::PUSH_Id(_) => 1,
            Self::PUSH_Iw(_) => 1,
            Self::PUSH_Md(_) => 1,
            Self::PUSH_Mw(_) => 1,
            Self::PUSH_Rd(_) => 1,
            Self::PUSH_Rw(_) => 1,
            Self::PUSH_SrO16(_) => 1,
            Self::PUSH_SrO32(_) => 1,
            Self::RCL_Mb_Cl(_) => 1,
            Self::RCL_Mb_Ib(_, _) => 2,
            Self::RCL_Md_Cl(_) => 1,
            Self::RCL_Md_Ib(_, _) => 2,
            Self::RCL_Mw_Cl(_) => 1,
            Self::RCL_Mw_Ib(_, _) => 2,
            Self::RCL_Rb_Cl(_) => 1,
            Self::RCL_Rb_Ib(_, _) => 2,
            Self::RCL_Rd_Cl(_) => 1,
            Self::RCL_Rd_Ib(_, _) => 2,
            Self::RCL_Rw_Cl(_) => 1,
            Self::RCL_Rw_Ib(_, _) => 2,
            Self::RCR_Mb_Cl(_) => 1,
            Self::RCR_Mb_Ib(_, _) => 2,
            Self::RCR_Md_Cl(_) => 1,
            Self::RCR_Md_Ib(_, _) => 2,
            Self::RCR_Mw_Cl(_) => 1,
            Self::RCR_Mw_Ib(_, _) => 2,
            Self::RCR_Rb_Cl(_) => 1,
            Self::RCR_Rb_Ib(_, _) => 2,
            Self::RCR_Rd_Cl(_) => 1,
            Self::RCR_Rd_Ib(_, _) => 2,
            Self::RCR_Rw_Cl(_) => 1,
            Self::RCR_Rw_Ib(_, _) => 2,
            Self::RDMSR => 0,
            Self::RDTSC => 0,
            Self::RDTSCP => 0,
            Self::REPNZ_CMPSB(_) => 1,
            Self::REPNZ_CMPSD(_) => 1,
            Self::REPNZ_CMPSW(_) => 1,
            Self::REPNZ_SCASB(_) => 1,
            Self::REPNZ_SCASD(_) => 1,
            Self::REPNZ_SCASW(_) => 1,
            Self::REPZ_CMPSB(_) => 1,
            Self::REPZ_CMPSD(_) => 1,
            Self::REPZ_CMPSW(_) => 1,
            Self::REPZ_SCASB(_) => 1,
            Self::REPZ_SCASD(_) => 1,
            Self::REPZ_SCASW(_) => 1,
            Self::REP_INSB(_) => 1,
            Self::REP_INSD(_) => 1,
            Self::REP_INSW(_) => 1,
            Self::REP_LODSB(_) => 1,
            Self::REP_LODSD(_) => 1,
            Self::REP_LODSW(_) => 1,
            Self::REP_MOVSB(_) => 1,
            Self::REP_MOVSD(_) => 1,
            Self::REP_MOVSW(_) => 1,
            Self::REP_OUTSB(_) => 1,
            Self::REP_OUTSD(_) => 1,
            Self::REP_OUTSW(_) => 1,
            Self::REP_STOSB(_) => 1,
            Self::REP_STOSD(_) => 1,
            Self::REP_STOSW(_) => 1,
            Self::RETF_D16(_) => 1,
            Self::RETF_D32(_) => 1,
            Self::RET_D16(_) => 1,
            Self::RET_D32(_) => 1,
            Self::ROL_Mb_Cl(_) => 1,
            Self::ROL_Mb_Ib(_, _) => 2,
            Self::ROL_Md_Cl(_) => 1,
            Self::ROL_Md_Ib(_, _) => 2,
            Self::ROL_Mw_Cl(_) => 1,
            Self::ROL_Mw_Ib(_, _) => 2,
            Self::ROL_Rb_Cl(_) => 1,
            Self::ROL_Rb_Ib(_, _) => 2,
            Self::ROL_Rd_Cl(_) => 1,
            Self::ROL_Rd_Ib(_, _) => 2,
            Self::ROL_Rw_Cl(_) => 1,
            Self::ROL_Rw_Ib(_, _) => 2,
            Self::ROR_Mb_Cl(_) => 1,
            Self::ROR_Mb_Ib(_, _) => 2,
            Self::ROR_Md_Cl(_) => 1,
            Self::ROR_Md_Ib(_, _) => 2,
            Self::ROR_Mw_Cl(_) => 1,
            Self::ROR_Mw_Ib(_, _) => 2,
            Self::ROR_Rb_Cl(_) => 1,
            Self::ROR_Rb_Ib(_, _) => 2,
            Self::ROR_Rd_Cl(_) => 1,
            Self::ROR_Rd_Ib(_, _) => 2,
            Self::ROR_Rw_Cl(_) => 1,
            Self::ROR_Rw_Ib(_, _) => 2,
            Self::RSM => 0,
            Self::SAHF => 0,
            Self::SALC => 0,
            Self::SAR_Mb_Cl(_) => 1,
            Self::SAR_Mb_Ib(_, _) => 2,
            Self::SAR_Md_Cl(_) => 1,
            Self::SAR_Md_Ib(_, _) => 2,
            Self::SAR_Mw_Cl(_) => 1,
            Self::SAR_Mw_Ib(_, _) => 2,
            Self::SAR_Rb_Cl(_) => 1,
            Self::SAR_Rb_Ib(_, _) => 2,
            Self::SAR_Rd_Cl(_) => 1,
            Self::SAR_Rd_Ib(_, _) => 2,
            Self::SAR_Rw_Cl(_) => 1,
            Self::SAR_Rw_Ib(_, _) => 2,
            Self::SBB_Mb_Ib(_, _) => 2,
            Self::SBB_Mb_Rb(_, _) => 2,
            Self::SBB_Md_Id(_, _) => 2,
            Self::SBB_Md_Rd(_, _) => 2,
            Self::SBB_Mw_Iw(_, _) => 2,
            Self::SBB_Mw_Rw(_, _) => 2,
            Self::SBB_Rb_Ib(_, _) => 2,
            Self::SBB_Rb_Mb(_, _) => 2,
            Self::SBB_Rb_Rb(_, _) => 2,
            Self::SBB_Rd_Id(_, _) => 2,
            Self::SBB_Rd_Md(_, _) => 2,
            Self::SBB_Rd_Rd(_, _) => 2,
            Self::SBB_Rw_Iw(_, _) => 2,
            Self::SBB_Rw_Mw(_, _) => 2,
            Self::SBB_Rw_Rw(_, _) => 2,
            Self::SCASB(_) => 1,
            Self::SCASD(_) => 1,
            Self::SCASW(_) => 1,
            Self::SETCC_Mb(_, _) => 2,
            Self::SETCC_Rb(_, _) => 2,
            Self::SGDT_Mp(_) => 1,
            Self::SHLD_Md_Rd_Cl(_, _) => 2,
            Self::SHLD_Md_Rd_Ib(_, _, _) => 3,
            Self::SHLD_Mw_Rw_Cl(_, _) => 2,
            Self::SHLD_Mw_Rw_Ib(_, _, _) => 3,
            Self::SHLD_Rd_Rd_Cl(_, _) => 2,
            Self::SHLD_Rd_Rd_Ib(_, _, _) => 3,
            Self::SHLD_Rw_Rw_Cl(_, _) => 2,
            Self::SHLD_Rw_Rw_Ib(_, _, _) => 3,
            Self::SHL_Mb_Cl(_) => 1,
            Self::SHL_Mb_Ib(_, _) => 2,
            Self::SHL_Md_Cl(_) => 1,
            Self::SHL_Md_Ib(_, _) => 2,
            Self::SHL_Mw_Cl(_) => 1,
            Self::SHL_Mw_Ib(_, _) => 2,
            Self::SHL_Rb_Cl(_) => 1,
            Self::SHL_Rb_Ib(_, _) => 2,
            Self::SHL_Rd_Cl(_) => 1,
            Self::SHL_Rd_Ib(_, _) => 2,
            Self::SHL_Rw_Cl(_) => 1,
            Self::SHL_Rw_Ib(_, _) => 2,
            Self::SHRD_Md_Rd_Cl(_, _) => 2,
            Self::SHRD_Md_Rd_Ib(_, _, _) => 3,
            Self::SHRD_Mw_Rw_Cl(_, _) => 2,
            Self::SHRD_Mw_Rw_Ib(_, _, _) => 3,
            Self::SHRD_Rd_Rd_Cl(_, _) => 2,
            Self::SHRD_Rd_Rd_Ib(_, _, _) => 3,
            Self::SHRD_Rw_Rw_Cl(_, _) => 2,
            Self::SHRD_Rw_Rw_Ib(_, _, _) => 3,
            Self::SHR_Mb_Cl(_) => 1,
            Self::SHR_Mb_Ib(_, _) => 2,
            Self::SHR_Md_Cl(_) => 1,
            Self::SHR_Md_Ib(_, _) => 2,
            Self::SHR_Mw_Cl(_) => 1,
            Self::SHR_Mw_Ib(_, _) => 2,
            Self::SHR_Rb_Cl(_) => 1,
            Self::SHR_Rb_Ib(_, _) => 2,
            Self::SHR_Rd_Cl(_) => 1,
            Self::SHR_Rd_Ib(_, _) => 2,
            Self::SHR_Rw_Cl(_) => 1,
            Self::SHR_Rw_Ib(_, _) => 2,
            Self::SIDT_Mp(_) => 1,
            Self::SIMD(_) => 1,
            Self::SIMD_M(_, _, _, _) => 4,
            Self::SIMD_R(_, _, _, _) => 4,
            Self::SIMD_S(_, _, _, _) => 4,
            Self::SLDT_Md(_) => 1,
            Self::SLDT_Mw(_) => 1,
            Self::SLDT_Rd(_) => 1,
            Self::SLDT_Rw(_) => 1,
            Self::SMSW_Mw(_) => 1,
            Self::SMSW_Rw(_) => 1,
            Self::STC => 0,
            Self::STD => 0,
            Self::STI => 0,
            Self::STOSB(_) => 1,
            Self::STOSD(_) => 1,
            Self::STOSW(_) => 1,
            Self::STR_Md(_) => 1,
            Self::STR_Mw(_) => 1,
            Self::STR_Rd(_) => 1,
            Self::STR_Rw(_) => 1,
            Self::SUB_Mb_Ib(_, _) => 2,
            Self::SUB_Mb_Rb(_, _) => 2,
            Self::SUB_Md_Id(_, _) => 2,
            Self::SUB_Md_Rd(_, _) => 2,
            Self::SUB_Mw_Iw(_, _) => 2,
            Self::SUB_Mw_Rw(_, _) => 2,
            Self::SUB_Rb_Ib(_, _) => 2,
            Self::SUB_Rb_Mb(_, _) => 2,
            Self::SUB_Rb_Rb(_, _) => 2,
            Self::SUB_Rd_Id(_, _) => 2,
            Self::SUB_Rd_Md(_, _) => 2,
            Self::SUB_Rd_Rd(_, _) => 2,
            Self::SUB_Rw_Iw(_, _) => 2,
            Self::SUB_Rw_Mw(_, _) => 2,
            Self::SUB_Rw_Rw(_, _) => 2,
            Self::SWAPGS => 0,
            Self::SYSCALL => 0,
            Self::SYSENTER => 0,
            Self::SYSEXIT => 0,
            Self::SYSRET => 0,
            Self::TEST_Mb_Ib(_, _) => 2,
            Self::TEST_Mb_Rb(_, _) => 2,
            Self::TEST_Md_Id(_, _) => 2,
            Self::TEST_Md_Rd(_, _) => 2,
            Self::TEST_Mw_Iw(_, _) => 2,
            Self::TEST_Mw_Rw(_, _) => 2,
            Self::TEST_Rb_Ib(_, _) => 2,
            Self::TEST_Rb_Rb(_, _) => 2,
            Self::TEST_Rd_Id(_, _) => 2,
            Self::TEST_Rd_Rd(_, _) => 2,
            Self::TEST_Rw_Iw(_, _) => 2,
            Self::TEST_Rw_Rw(_, _) => 2,
            Self::UD => 0,
            Self::VERR_Mw(_) => 1,
            Self::VERR_Rw(_) => 1,
            Self::VERW_Mw(_) => 1,
            Self::VERW_Rw(_) => 1,
            Self::WAIT => 0,
            Self::WBINVD => 0,
            Self::WRMSR => 0,
            Self::XADD_Mb_Rb(_, _) => 2,
            Self::XADD_Md_Rd(_, _) => 2,
            Self::XADD_Mw_Rw(_, _) => 2,
            Self::XADD_Rb_Rb(_, _) => 2,
            Self::XADD_Rd_Rd(_, _) => 2,
            Self::XADD_Rw_Rw(_, _) => 2,
            Self::XCHG_Mb_Rb(_, _) => 2,
            Self::XCHG_Md_Rd(_, _) => 2,
            Self::XCHG_Mw_Rw(_, _) => 2,
            Self::XCHG_Rb_Rb(_, _) => 2,
            Self::XCHG_Rd_Rd(_, _) => 2,
            Self::XCHG_Rw_Rw(_, _) => 2,
            Self::XGETBV => 0,
            Self::XLAT_Mb(_) => 1,
            Self::XOR_Mb_Ib(_, _) => 2,
            Self::XOR_Mb_Rb(_, _) => 2,
            Self::XOR_Md_Id(_, _) => 2,
            Self::XOR_Md_Rd(_, _) => 2,
            Self::XOR_Mw_Iw(_, _) => 2,
            Self::XOR_Mw_Rw(_, _) => 2,
            Self::XOR_Rb_Ib(_, _) => 2,
            Self::XOR_Rb_Mb(_, _) => 2,
            Self::XOR_Rb_Rb(_, _) => 2,
            Self::XOR_Rd_Id(_, _) => 2,
            Self::XOR_Rd_Md(_, _) => 2,
            Self::XOR_Rd_Rd(_, _) => 2,
            Self::XOR_Rw_Iw(_, _) => 2,
            Self::XOR_Rw_Mw(_, _) => 2,
            Self::XOR_Rw_Rw(_, _) => 2,
            Self::XSETBV => 0,
        }
    }

    /// Returns `true` for an MMX, SSE or SSE2 form (`SIMD`, `SIMD_R`, `SIMD_M` or `SIMD_S`).
    #[inline]
    pub const fn is_simd(&self) -> bool {
        matches!(
            self,
            Self::SIMD(..) | Self::SIMD_R(..) | Self::SIMD_M(..) | Self::SIMD_S(..)
        )
    }
}

/// Well Known Mnemonic for x86 instructions
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mnemonic {
    AAA,
    AAD,
    AAM,
    AAS,
    ADC,
    ADD,
    AND,
    ARPL,
    BOUND,
    BSF,
    BSR,
    BSWAP,
    BT,
    BTC,
    BTR,
    BTS,
    CALL,
    CALLF,
    CDQ,
    CLC,
    CLD,
    CLI,
    CLTS,
    CMC,
    CMOV,
    CMP,
    CMPS,
    CMPXCHG,
    CPUID,
    CWD,
    DAA,
    DAS,
    DEC,
    DIV,
    ENTER,
    ESC,
    HLT,
    ICEBP,
    IDIV,
    IMUL,
    IN,
    INC,
    INS,
    INT,
    INT3,
    INTO,
    INVD,
    INVLPG,
    IRET,
    IRETD,
    JCC,
    JCXZ,
    JECXZ,
    JMP,
    JMPF,
    LAHF,
    LAR,
    LDS,
    LEA,
    LEAVE,
    LES,
    LFS,
    LGDT,
    LGS,
    LIDT,
    LLDT,
    LMSW,
    LODS,
    LOOP,
    LOOPNZ,
    LOOPZ,
    LSL,
    LSS,
    LTR,
    MOV,
    MOVS,
    MOVSX,
    MOVZX,
    MUL,
    NEG,
    NOP,
    NOT,
    OR,
    OUT,
    OUTS,
    PAUSE,
    POP,
    POPA,
    POPAD,
    POPCNT,
    POPF,
    POPFD,
    PUSH,
    PUSHA,
    PUSHAD,
    PUSHF,
    PUSHFD,
    RCL,
    RCR,
    RDMSR,
    RDTSC,
    RDTSCP,
    RET,
    RETF,
    ROL,
    ROR,
    RSM,
    SAHF,
    SALC,
    SAR,
    SBB,
    SCAS,
    SETCC,
    SGDT,
    SHL,
    SHLD,
    SHR,
    SHRD,
    SIDT,
    SIMD,
    SLDT,
    SMSW,
    STC,
    STD,
    STI,
    STOS,
    STR,
    SUB,
    SWAPGS,
    SYSCALL,
    SYSENTER,
    SYSEXIT,
    SYSRET,
    TEST,
    UD,
    VERR,
    VERW,
    WAIT,
    WBINVD,
    WRMSR,
    XADD,
    XCHG,
    XGETBV,
    XLAT,
    XOR,
    XSETBV,
}
