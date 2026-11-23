//! User Mode Emulator interface trait

use crate::_prelude_::*;
use crate::cpu::ProcessorState;
use crate::flat::FlatUME;
use crate::harvard::HarvardUME;

/// Creates a new HarvardUME instance.
///
/// # Parameters
/// - `code`: The code segment as a boxed slice of bytes.
/// - `data`: The data segment as a boxed slice of bytes.
/// - `entry`: The entry point offset.
/// - `stack_pointer`: The initial stack pointer offset.
///
/// ## Limitation
/// * The code segment and data segment must not overlap.
/// * During execution, it is not possible to access the segment registers.
/// * Some 16bit features are not implemented.
#[inline]
pub fn harvard(
    code: Box<[u8]>,
    data: Box<[u8]>,
    entry: Offset32,
    stack_pointer: Offset32,
) -> HarvardUME {
    HarvardUME::new(code, data, entry, stack_pointer)
}

/// Creates a new FlatUME instance.
///
/// # Parameters
/// - `memory`: The guest memory as a vector of bytes.
/// - `guest_base`: The base address of the guest memory.
/// - `entry`: The entry point offset.
/// - `stack_pointer`: The initial stack pointer offset.
///
/// ## Limitation
/// * Some 16bit features are not implemented.
#[inline]
pub fn flat(
    memory: impl Into<Vec<u8>>,
    guest_base: Offset32,
    entry: Offset32,
    stack_pointer: Offset32,
) -> Result<FlatUME, crate::flat::InitError> {
    FlatUME::new(memory.into(), guest_base, entry, stack_pointer)
}

/// User Mode Emulator interface trait
pub trait Emulator {
    /// Returns a reference to the processor state.
    fn state(&self) -> &ProcessorState;

    /// Returns a mutable reference to the processor state.
    fn state_mut(&mut self) -> &mut ProcessorState;

    /// Executes the emulation
    fn execute(&mut self) -> Result<(), Exception>;

    /// Resumes execution after a pause (e.g., after a syscall)
    fn resume_next(&mut self);

    /// Returns `true` if the emulator is using flat memory mode, `false` otherwise.
    fn is_flat(&self) -> bool;

    /// Converts the emulator to a semi-flat memory mode emulator if supported.
    fn to_semi_flat(&mut self) -> Result<crate::flat::FlatUME, Exception>;

    /// Returns a reference to the emulated memory.
    fn memory(&self) -> (&[u8], Offset32);

    /// Adjusts the program break (end of the data segment) to the specified address.
    /// Zero queries the current break. Success returns the resulting break;
    /// failure returns the unchanged break. Flat memory supports growth only,
    /// zero-initializes new bytes, and treats the break as the exclusive memory end.
    /// An exclusive end of 2^32 is represented as zero.
    fn brk(&mut self, address: u32) -> Result<u32, u32>;

    /// Writes the given bytes into the emulated memory at the specified address.
    fn write_memory(&mut self, address: u32, bytes: &[u8]) -> Result<(), Exception>;

    /// Fills the emulated memory at the specified address with the given value for the specified size.
    fn fill_memory(&mut self, address: u32, value: u8, size: usize) -> Result<(), Exception>;

    /// Reads a slice of memory from the specified address with the given size.
    fn read_memory(&self, address: u32, size: usize) -> Result<&[u8], Exception> {
        let (memory, base_offset) = self.memory();
        let base_offset = base_offset.0 as usize;
        let base = (address as usize)
            .checked_sub(base_offset)
            .ok_or(Exception::SegmentationViolation(Offset32(address)))?;
        let end = base
            .checked_add(size)
            .ok_or(Exception::SegmentationViolation(Offset32(address)))? as usize;
        if base >= memory.len() || end > memory.len() {
            return Err(Exception::SegmentationViolation(Offset32(address)));
        }
        Ok(&memory[base..end])
    }

    /// Reads a null-terminated string from the specified offset without encoding.
    fn get_asciz_noenc(&self, address: u32) -> Result<&[u8], Exception> {
        let (memory, base_offset) = self.memory();
        let base_offset = base_offset.0 as usize;
        let base = (address as usize)
            .checked_sub(base_offset)
            .ok_or(Exception::SegmentationViolation(Offset32(address)))?;
        let mut end = base;
        while end < memory.len() && memory[end] != 0 {
            end += 1;
        }
        if end == memory.len() {
            return Err(Exception::SegmentationViolation(Offset32(address)));
        }
        Ok(&memory[base..end])
    }

    /// Reads a null-terminated string from the specified offset and attempts to convert it to a UTF-8 `String`.
    fn get_cstring(&self, address: u32) -> Result<Option<String>, Exception> {
        self.get_asciz_noenc(address)
            .map(|bytes| match String::from_utf8(bytes.to_vec()) {
                Ok(s) => Some(s),
                Err(_) => None,
            })
    }
}

/// Exception Code
#[derive(Debug, Clone, Copy)]
pub enum Exception {
    DivisionError,
    Unimplemented(crate::harvard::uop::Uop),
    Unimplemented2(crate::flat::uop::Uop),
    StackFault,
    SegmentationViolation(Offset32),
    OutOfCode,
    RdTsc,
    Swi(u8),
    SysEnter,
    SysCall,
    /// MOV Sreg, r/m: the host validates the selector, updates `sr` and resumes.
    LoadSegment(SrIndex, u16),
    /// #UD: an encoding the emulated CPU does not define. Valid encodings UME
    /// does not implement stop as `Unimplemented` / `Unimplemented2` instead.
    InvalidOpcode,
    /// #GP(0) from an implemented instruction, such as a misaligned 16-byte
    /// SSE operand or a reserved MXCSR bit. Defined with or without `sse`, so
    /// exhaustive host matches do not depend on the feature.
    GeneralProtection,
}
