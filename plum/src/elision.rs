//! Trace-local liveness of CF and the five remaining arithmetic flags.
use crate::prelude::*;

#[derive(Clone, Copy)]
pub(crate) enum Effect {
    Full,
    PreserveCarry,
    ReadCarry,
    Compare,
    MayWrite,
    Transparent,
    Boundary,
}

pub(crate) fn effect(ir: IrOp) -> Effect {
    use IrOp::*;
    match ir {
        ADD_Rd_Rd(..) | ADD_Rd_Id(..) | ADD_Rw_Rw(..) | ADD_Rw_Iw(..) | ADD_Rb_Rb(..)
        | ADD_Rb_Ib(..) | SUB_Rd_Rd(..) | SUB_Rd_Id(..) | SUB_Rw_Rw(..) | SUB_Rw_Iw(..)
        | SUB_Rb_Rb(..) | SUB_Rb_Ib(..) | AND_Rd_Rd(..) | AND_Rd_Id(..) | AND_Rw_Rw(..)
        | AND_Rw_Iw(..) | AND_Rb_Rb(..) | AND_Rb_Ib(..) | OR_Rd_Rd(..) | OR_Rd_Id(..)
        | OR_Rw_Rw(..) | OR_Rw_Iw(..) | OR_Rb_Rb(..) | OR_Rb_Ib(..) | XOR_Rd_Rd(..)
        | XOR_Rd_Id(..) | XOR_Rw_Rw(..) | XOR_Rw_Iw(..) | XOR_Rb_Rb(..) | XOR_Rb_Ib(..) => {
            Effect::Full
        }
        CMP_Rd_Rd(..) | CMP_Rd_Id(..) | CMP_Rw_Rw(..) | CMP_Rw_Iw(..) | CMP_Rb_Rb(..)
        | CMP_Rb_Ib(..) | TEST_Rd_Rd(..) | TEST_Rd_Id(..) | TEST_Rw_Rw(..) | TEST_Rw_Iw(..)
        | TEST_Rb_Rb(..) | TEST_Rb_Ib(..) => Effect::Compare,
        ADC_Rd_Rd(..) | ADC_Rd_Id(..) | SBB_Rd_Rd(..) | SBB_Rd_Id(..) => Effect::ReadCarry,
        SHL_Rd_Ib(..) | SHR_Rd_Ib(..) | SAR_Rd_Ib(..) | SHL_Rd_Cl(..) | SHR_Rd_Cl(..)
        | SAR_Rd_Cl(..) | IMUL_Rd_Rd(..) | IMUL_Rd_Rd_Id(..) => Effect::MayWrite,
        NEG_Rd(..) => Effect::Full,
        INC_Rd(..) | INC_Rw(..) | INC_Rb(..) | DEC_Rd(..) | DEC_Rw(..) | DEC_Rb(..) => {
            Effect::PreserveCarry
        }
        MOV_Rd_Rd(..)
        | MOV_Rd_Id(..)
        | MOV_Rw_Rw(..)
        | MOV_Rw_Iw(..)
        | MOV_Rb_Rb(..)
        | MOV_Rb_Ib(..)
        | MOVZX_Rd_Rb(..)
        | MOVZX_Rd_Rw(..)
        | MOVZX_Rw_Rb(..)
        | MOVSX_Rd_Rb(..)
        | MOVSX_Rd_Rw(..)
        | MOVSX_Rw_Rb(..)
        | LEA_Rd_Md(_, MemOpr::A32(_))
        | NOP
        | PAUSE => Effect::Transparent,
        _ => Effect::Boundary,
    }
}

pub(crate) fn analyze(effects: &[(u32, Effect)], mut candidate: impl FnMut(u32)) {
    // Bit 0 is CF. All whitelisted operations treat the other five flags alike.
    let mut live = 3u8;
    for &(index, effect) in effects.iter().rev() {
        let writes = match effect {
            Effect::Full | Effect::ReadCarry | Effect::MayWrite => 3,
            // CMP/TEST consume no flags, and their own Uops stay intact.
            Effect::Compare => {
                live = 0;
                continue;
            }
            Effect::PreserveCarry => 2,
            Effect::Transparent => continue,
            Effect::Boundary => {
                live = 3;
                continue;
            }
        };
        if writes & live == 0 {
            candidate(index);
        }
        // Zero shift counts preserve flags; IMUL defines only CF and OF.
        // Neither proves a full overwrite of the flags of its predecessor.
        if !matches!(effect, Effect::MayWrite) {
            live &= !writes;
        }
        if matches!(effect, Effect::ReadCarry) {
            live |= 1;
        }
    }
}
