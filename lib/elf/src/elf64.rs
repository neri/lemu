use core::mem::transmute;

use super::*;

pub type ElfAddr = u64;
pub type ElfOff = u64;

pub struct Elf64<'a> {
    slice: &'a [u8],
}

impl<'a> Elf64<'a> {
    #[inline]
    pub fn identify(slice: &'a [u8]) -> Option<Self> {
        match ElfFormat::identify(slice, None, None) {
            Some(ElfFormat::Elf64) => Some(Self { slice }),
            _ => None,
        }
    }

    #[inline]
    pub fn header(&self) -> &Header {
        unsafe { transmute(self.slice.as_ptr()) }
    }

    #[inline]
    pub fn segments(&self) -> Option<impl Iterator<Item = &ProgramHeader>> {
        self.slice
            .get(self.header().e_phoff as usize..)
            .and_then(|data| {
                EntryReader::new(
                    data,
                    self.header().e_phentsize as usize,
                    self.header().e_phnum as usize,
                )
            })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub e_ident: [u8; EI_NIDENT],
    pub e_type: ElfType,
    pub e_machine: Machine,
    pub e_version: ElfWord,
    pub e_entry: ElfAddr,
    pub e_phoff: ElfOff,
    pub e_shoff: ElfOff,
    pub e_flags: ElfWord,
    pub e_ehsize: ElfHalf,
    pub e_phentsize: ElfHalf,
    pub e_phnum: ElfHalf,
    pub e_shentsize: ElfHalf,
    pub e_shnum: ElfHalf,
    pub e_shstrndx: ElfHalf,
}

impl Header {
    #[inline]
    pub fn from_slice<'a>(slice: &'a [u8]) -> Option<&'a Self> {
        if slice.len() < core::mem::size_of::<Self>() {
            None
        } else {
            // SAFETY: The size is checked above.
            Some(unsafe { &*(slice.as_ptr() as *const Self) })
        }
    }

    #[inline]
    pub fn is_valid_type(&self, elf_type: ElfType, machine: Machine) -> bool {
        self.e_type == elf_type && self.e_machine == machine
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProgramHeader {
    pub p_type: SegmentType,
    pub p_flags: SegmentFlags,
    pub p_offset: ElfOff,
    pub p_vaddr: ElfAddr,
    pub p_paddr: ElfAddr,
    pub p_filesz: ElfXWord,
    pub p_memsz: ElfXWord,
    pub p_align: ElfXWord,
}
