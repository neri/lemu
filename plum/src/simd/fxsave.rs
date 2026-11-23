//! The 512-byte FXSAVE/FXRSTOR image (32-bit format, SDM Vol. 1, 10.5.1).
//!
//! | bytes   | field                                                  |
//! | ------- | ------------------------------------------------------ |
//! | 0-1     | FCW                                                    |
//! | 2-3     | FSW                                                    |
//! | 4       | abridged FTW: bit n = 1 when physical register Rn is not empty |
//! | 6-7     | FOP                                                    |
//! | 8-11    | FIP (offset), 12-13 FCS                                |
//! | 16-19   | FDP (offset), 20-21 FDS                                |
//! | 24-27   | MXCSR, 28-31 MXCSR_MASK                                |
//! | 32-159  | ST0-ST7 (MM0-MM7 as R0-R7 seen from TOP), 16 bytes each |
//! | 160-287 | XMM0-XMM7                                              |
//! | 288-463 | reserved                                               |
//! | 464-511 | available to software                                  |
//!
//! FXSAVE writes bytes 0-463, reserved fields as zeros, and leaves 464-511 as
//! they were; FXRSTOR ignores the reserved fields, MXCSR_MASK and 464-511.
//! The pointers are offsets with selectors in every mode, real and
//! virtual-8086 mode included. OSFXSR does not change the image: XMM and
//! MXCSR are always saved and restored.
use super::{MXCSR_EXCEPTION_MASKS, MXCSR_MASK, SimdFault};
use crate::cpu::ProcessorState;
use crate::fpu::{FpuValue, Raw80};

/// One FXSAVE/FXRSTOR memory operand.
pub(crate) type FxImage = [u8; 512];
/// Bytes FXSAVE stores; 464-511 belong to software.
pub(crate) const FXSAVE_WRITTEN: usize = 464;

fn put(bytes: &mut [u8], offset: usize, value: &[u8]) {
    bytes[offset..offset + value.len()].copy_from_slice(value);
}
fn word(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
fn dword(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

/// The image FXSAVE stores. `opcode` is FOP, which HarvardUME resolves lazily.
pub(crate) fn fxsave(state: &ProcessorState, opcode: u16) -> FxImage {
    let fpu = &state.fpu;
    let mut bytes = [0; 512];
    put(&mut bytes, 0, &fpu.control_word().to_le_bytes());
    put(&mut bytes, 2, &fpu.status_word().to_le_bytes());
    bytes[4] = (0..8)
        .filter(|&n| (fpu.tags >> (n * 2)) & 3 != 3)
        .fold(0, |tags, n| tags | 1 << n);
    put(&mut bytes, 6, &(opcode & 0x7ff).to_le_bytes());
    put(&mut bytes, 8, &fpu.instruction_pointer.to_le_bytes());
    put(&mut bytes, 12, &fpu.instruction_selector.to_le_bytes());
    put(&mut bytes, 16, &fpu.data_pointer.to_le_bytes());
    put(&mut bytes, 20, &fpu.data_selector.to_le_bytes());
    put(&mut bytes, 24, &state.simd.mxcsr.to_le_bytes());
    put(&mut bytes, 28, &MXCSR_MASK.to_le_bytes());
    for i in 0..8u8 {
        let raw = fpu.registers[fpu.physical(i)].raw();
        let at = 32 + usize::from(i) * 16;
        put(&mut bytes, at, &raw.significand.to_le_bytes());
        put(&mut bytes, at + 8, &raw.sign_exponent.to_le_bytes());
    }
    for (n, xmm) in state.simd.xmm.iter().enumerate() {
        put(&mut bytes, 160 + n * 16, xmm);
    }
    bytes
}

/// Restores from an FXSAVE image; on a fault nothing changes. A set bit
/// outside MXCSR_MASK is #GP(0). `unmasked_ok` is false on HarvardUME and
/// FlatUME, which reject an image with an unmasked x87 or SIMD exception, as
/// they reject FLDCW and LDMXCSR. The engine recomputes FSW.ES/B if it models
/// them.
pub(crate) fn fxrstor(
    state: &mut ProcessorState,
    bytes: &FxImage,
    unmasked_ok: bool,
) -> Result<(), SimdFault> {
    let mxcsr = dword(bytes, 24);
    if mxcsr & !MXCSR_MASK != 0 {
        return Err(SimdFault::GeneralProtection);
    }
    let control = word(bytes, 0);
    if !unmasked_ok
        && (control & 0x3f != 0x3f || mxcsr & MXCSR_EXCEPTION_MASKS != MXCSR_EXCEPTION_MASKS)
    {
        return Err(SimdFault::Unimplemented);
    }
    let status = word(bytes, 2);
    let fpu = &mut state.fpu;
    fpu.control = control;
    fpu.top = ((status >> 11) & 7) as u8;
    fpu.status = status & !0x3800;
    fpu.last_opcode = word(bytes, 6) & 0x7ff;
    fpu.opcode_pending = false;
    fpu.instruction_pointer = dword(bytes, 8);
    fpu.instruction_selector = word(bytes, 12);
    fpu.data_pointer = dword(bytes, 16);
    fpu.data_selector = word(bytes, 20);
    for i in 0..8u8 {
        let at = 32 + usize::from(i) * 16;
        let raw = Raw80 {
            significand: u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap()),
            sign_exponent: word(bytes, at + 8),
        };
        let physical = fpu.physical(i);
        fpu.registers[physical] = FpuValue::Raw80(raw);
    }
    // Rebuild the two-bit tags from the abridged FTW and the restored values:
    // empty, or the class of the value (valid, zero, special).
    let abridged = bytes[4];
    fpu.tags = (0..8).fold(0, |tags, n| {
        let tag = if abridged & (1 << n) == 0 {
            3
        } else {
            fpu.registers[n].tag()
        };
        tags | (tag << (n * 2))
    });
    state.simd.mxcsr = mxcsr;
    for (n, xmm) in state.simd.xmm.iter_mut().enumerate() {
        xmm.copy_from_slice(&bytes[160 + n * 16..176 + n * 16]);
    }
    Ok(())
}
