//! Small, in-process Linux/i386 ABI harness for static freestanding ELFs.
//! No host syscall forwarding. This is not a Linux kernel or a memory sandbox.
use std::collections::BTreeMap;

use plum::flat::FlatUME;
use plum::gpr::PartialRegister;
use plum::prelude::*;

const PAGE: u32 = 4096;
const RAM: usize = 32 * 1024 * 1024;
const STACK: u32 = 1024 * 1024;
const MAX_OUTPUT: usize = 1024 * 1024;

pub struct Report {
    pub exit_code: u8,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub syscalls: BTreeMap<u32, u64>,
    pub cached_uops: usize,
}

struct Segment {
    address: u32,
    size: u32,
    offset: usize,
    file_size: usize,
    executable: bool,
}

struct Image {
    memory: Box<[u8]>,
    base: u32,
    entry: u32,
    esp: u32,
    heap: u32,
    heap_limit: u32,
}

fn bytes(data: &[u8], offset: usize, len: usize) -> Result<&[u8], String> {
    data.get(offset..offset.checked_add(len).ok_or("ELF range overflow")?)
        .ok_or_else(|| "ELF file is truncated".into())
}
fn word(data: &[u8], offset: usize) -> Result<u16, String> {
    Ok(u16::from_le_bytes(
        bytes(data, offset, 2)?.try_into().unwrap(),
    ))
}
fn dword(data: &[u8], offset: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        bytes(data, offset, 4)?.try_into().unwrap(),
    ))
}
fn align(value: u32) -> Option<u32> {
    value.checked_add(PAGE - 1).map(|v| v & !(PAGE - 1))
}

fn load(elf: &[u8]) -> Result<Image, String> {
    bytes(elf, 0, 52)?;
    if &elf[..7] != b"\x7fELF\x01\x01\x01"
        || word(elf, 16)? != 2
        || word(elf, 18)? != 3
        || dword(elf, 20)? != 1
        || word(elf, 40)? != 52
        || word(elf, 42)? != 32
    {
        return Err("requires a little-endian static ELF32 ET_EXEC for i386".into());
    }
    let entry = dword(elf, 24)?;
    let phoff = dword(elf, 28)? as usize;
    let count = word(elf, 44)? as usize;
    if count == 0 || count > 128 {
        return Err("ELF program-header count must be 1..128".into());
    }
    bytes(elf, phoff, count * 32)?;
    let mut segments = Vec::new();
    for index in 0..count {
        let p = phoff + index * 32;
        match dword(elf, p)? {
            2 | 3 => return Err("dynamic linking and PT_INTERP are unsupported".into()),
            1 => {
                let offset = dword(elf, p + 4)? as usize;
                let address = dword(elf, p + 8)?;
                let file_size = dword(elf, p + 16)? as usize;
                let size = dword(elf, p + 20)?;
                if file_size > size as usize || address.checked_add(size).is_none() {
                    return Err("invalid PT_LOAD size or address overflow".into());
                }
                bytes(elf, offset, file_size)?;
                if size != 0 {
                    segments.push(Segment {
                        address,
                        size,
                        offset,
                        file_size,
                        executable: dword(elf, p + 24)? & 1 != 0,
                    });
                }
            }
            _ => {}
        }
    }
    let base = segments
        .iter()
        .map(|s| s.address)
        .min()
        .ok_or("ELF has no PT_LOAD")?
        & !(PAGE - 1);
    let end = base
        .checked_add(RAM as u32)
        .ok_or("guest RAM address overflow")?;
    let heap_limit = end - STACK;
    let loaded_end = segments.iter().map(|s| s.address + s.size).max().unwrap();
    let heap = align(loaded_end).ok_or("ELF heap address overflow")?;
    if heap > heap_limit
        || !segments
            .iter()
            .any(|s| s.executable && entry >= s.address && entry < s.address + s.size)
    {
        return Err("ELF exceeds RAM or entry is outside executable PT_LOAD".into());
    }
    let mut memory = vec![0; RAM].into_boxed_slice();
    for segment in segments {
        let start = (segment.address - base) as usize;
        memory[start..start + segment.file_size]
            .copy_from_slice(&elf[segment.offset..segment.offset + segment.file_size]);
        memory[start + segment.file_size..start + segment.size as usize].fill(0);
    }
    // Minimal Linux entry stack: argc/argv, empty envp, and basic auxiliary vector.
    let name = b"plum-guest\0";
    let name_offset = RAM - 64;
    memory[name_offset..name_offset + name.len()].copy_from_slice(name);
    let name_address = base + name_offset as u32;
    let words = [1u32, name_address, 0, 0, 6, PAGE, 9, entry, 0, 0];
    let stack_offset = (name_offset - words.len() * 4) & !15;
    for (i, value) in words.iter().enumerate() {
        memory[stack_offset + i * 4..stack_offset + i * 4 + 4]
            .copy_from_slice(&value.to_le_bytes());
    }
    Ok(Image {
        memory,
        base,
        entry,
        esp: base + stack_offset as u32,
        heap,
        heap_limit,
    })
}

fn errno(value: u32) -> u32 {
    0u32.wrapping_sub(value)
}
fn range(cpu: &FlatUME, base: u32, address: u32, len: usize) -> Option<std::ops::Range<usize>> {
    let start = address.checked_sub(base)? as usize;
    let end = start.checked_add(len)?;
    (end <= cpu.memory().len()).then_some(start..end)
}

pub fn run(elf: &[u8], input: &[u8]) -> Result<Report, String> {
    let image = load(elf)?;
    let mut heap = image.heap;
    let mut cpu = plum::ume::flat(
        image.memory,
        Offset32(image.base),
        Offset32(image.entry),
        Offset32(image.esp),
    )
    .map_err(|e| format!("guest init: {e:?}"))?;
    let mut report = Report {
        exit_code: 0,
        stdout: Vec::new(),
        stderr: Vec::new(),
        syscalls: BTreeMap::new(),
        cached_uops: 0,
    };
    let mut consumed = 0;
    for _ in 0..4096 {
        match cpu.execute() {
            Err(Exception::Swi(0x80)) => {}
            other => {
                return Err(format!(
                    "guest stopped at {:#010x}: {other:?}",
                    cpu.state().eip().read()
                ));
            }
        }
        let number = cpu.state().eax().read();
        let a = cpu.state().ebx().read();
        let b = cpu.state().ecx().read();
        let c = cpu.state().edx().read();
        *report.syscalls.entry(number).or_default() += 1;
        let result = match number {
            1 | 252 => {
                report.exit_code = a as u8;
                report.cached_uops = cpu.tracer().uop_cache().len();
                return Ok(report);
            }
            3 => {
                if a != 0 {
                    errno(9)
                } else if range(&cpu, image.base, b, c as usize).is_none() {
                    errno(14)
                } else {
                    let count = (c as usize).min(input.len() - consumed);
                    // This can target cached code; always use the tracked host API.
                    cpu.write_bytes(b, &input[consumed..consumed + count])
                        .map_err(|e| format!("guest read: {e:?}"))?;
                    consumed += count;
                    count as u32
                }
            }
            4 => {
                if a != 1 && a != 2 {
                    errno(9)
                } else if let Some(span) = range(&cpu, image.base, b, c as usize) {
                    let out = if a == 1 {
                        &mut report.stdout
                    } else {
                        &mut report.stderr
                    };
                    if out.len() + span.len() > MAX_OUTPUT {
                        errno(28)
                    } else {
                        out.extend_from_slice(&cpu.memory()[span]);
                        c
                    }
                } else {
                    errno(14)
                }
            }
            192 => {
                // Only anonymous, private, RWX mappings, without MAP_FIXED.
                let flags = cpu.state().esi().read();
                let offset = cpu.state().ebp().read();
                if b == 0 || c != 7 || flags != 0x22 || offset != 0 {
                    errno(22)
                } else if let Some(end) = align(b)
                    .and_then(|len| heap.checked_add(len))
                    .filter(|end| *end <= image.heap_limit)
                {
                    let address = heap;
                    heap = end;
                    // mmap must zero the allocation even if a guest previously
                    // accessed the backing arena outside logical mappings.
                    let zero = [0; PAGE as usize];
                    for page in (address..end).step_by(PAGE as usize) {
                        cpu.write_bytes(page, &zero)
                            .map_err(|e| format!("guest mmap: {e:?}"))?;
                    }
                    address
                } else {
                    errno(12)
                }
            }
            _ => errno(38), // ENOSYS; do not pretend to support other Linux APIs.
        };
        cpu.state().eax().write(result);
        cpu.resume_next();
    }
    Err("guest exceeded the 4096-syscall limit".into())
}
