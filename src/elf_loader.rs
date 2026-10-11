// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::ext4::FsError;
use crate::sv39::{PTE_R, PTE_U, PTE_W, PTE_X};
use crate::vfs::Vfs;

const ELF_HEADER_SIZE: usize = 64;
const PROGRAM_HEADER_SIZE: usize = 56;
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_LSB: u8 = 1;
const ELF_VERSION_CURRENT: u32 = 1;
const ELF_TYPE_EXEC: u16 = 2;
const ELF_TYPE_DYN: u16 = 3;
const ELF_MACHINE_RISCV: u16 = 243;
const PT_LOAD: u32 = 1;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;
const MAX_LOAD_SEGMENTS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    TooSmall,
    BadMagic,
    UnsupportedClass,
    UnsupportedEndian,
    UnsupportedVersion,
    UnsupportedType,
    UnsupportedMachine,
    BadHeader,
    BadProgramHeader,
    SegmentOutOfRange,
    TooManySegments,
    NoLoadSegments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitLoadError {
    Fs(FsError),
    Elf(ElfError),
}

impl From<FsError> for InitLoadError {
    fn from(value: FsError) -> Self {
        Self::Fs(value)
    }
}

impl From<ElfError> for InitLoadError {
    fn from(value: ElfError) -> Self {
        Self::Elf(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfFileType {
    Executable,
    SharedObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfHeader {
    pub file_type: ElfFileType,
    pub entry: u64,
    pub phoff: u64,
    pub phentsize: u16,
    pub phnum: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgramHeader {
    pub p_type: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub align: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadSegment<'a> {
    pub vaddr: usize,
    pub file_size: usize,
    pub mem_size: usize,
    pub flags: u32,
    pub data: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct LoadedUserImage<'a> {
    pub entry: usize,
    pub stack_top: usize,
    pub segments: [Option<LoadSegment<'a>>; MAX_LOAD_SEGMENTS],
    pub segment_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedObjectLoadPlan {
    pub preferred_base: usize,
    pub entry_offset: usize,
    pub segment_count: usize,
    pub needs_relocation: bool,
}

pub fn parse_elf_header(bytes: &[u8]) -> Result<ElfHeader, ElfError> {
    if bytes.len() < ELF_HEADER_SIZE {
        return Err(ElfError::TooSmall);
    }
    if &bytes[0..4] != b"\x7fELF" {
        return Err(ElfError::BadMagic);
    }
    if bytes[4] != ELF_CLASS_64 {
        return Err(ElfError::UnsupportedClass);
    }
    if bytes[5] != ELF_DATA_LSB {
        return Err(ElfError::UnsupportedEndian);
    }
    if le32(bytes, 20) != ELF_VERSION_CURRENT {
        return Err(ElfError::UnsupportedVersion);
    }

    let file_type = match le16(bytes, 16) {
        ELF_TYPE_EXEC => ElfFileType::Executable,
        ELF_TYPE_DYN => ElfFileType::SharedObject,
        _ => return Err(ElfError::UnsupportedType),
    };
    if le16(bytes, 18) != ELF_MACHINE_RISCV {
        return Err(ElfError::UnsupportedMachine);
    }

    let phentsize = le16(bytes, 54);
    if phentsize as usize != PROGRAM_HEADER_SIZE {
        return Err(ElfError::BadHeader);
    }

    Ok(ElfHeader {
        file_type,
        entry: le64(bytes, 24),
        phoff: le64(bytes, 32),
        phentsize,
        phnum: le16(bytes, 56),
    })
}

pub fn parse_program_header(bytes: &[u8], hdr: &ElfHeader, index: usize) -> Result<ProgramHeader, ElfError> {
    if index >= hdr.phnum as usize {
        return Err(ElfError::BadProgramHeader);
    }

    let start = hdr.phoff as usize + index * hdr.phentsize as usize;
    let end = start + hdr.phentsize as usize;
    let ph = bytes.get(start..end).ok_or(ElfError::BadProgramHeader)?;

    Ok(ProgramHeader {
        p_type: le32(ph, 0),
        flags: le32(ph, 4),
        offset: le64(ph, 8),
        vaddr: le64(ph, 16),
        filesz: le64(ph, 32),
        memsz: le64(ph, 40),
        align: le64(ph, 48),
    })
}

pub fn pflags_to_pte(flags: u32) -> u64 {
    let mut pte = PTE_U;
    if flags & PF_R != 0 {
        pte |= PTE_R;
    }
    if flags & PF_W != 0 {
        pte |= PTE_W;
    }
    if flags & PF_X != 0 {
        pte |= PTE_X;
    }
    pte
}

pub fn load_user_elf(bytes: &[u8], stack_top: usize) -> Result<LoadedUserImage<'_>, ElfError> {
    let hdr = parse_elf_header(bytes)?;
    let mut image = LoadedUserImage {
        entry: hdr.entry as usize,
        stack_top,
        segments: [None; MAX_LOAD_SEGMENTS],
        segment_count: 0,
    };

    for index in 0..hdr.phnum as usize {
        let ph = parse_program_header(bytes, &hdr, index)?;
        if ph.p_type != PT_LOAD {
            continue;
        }
        if ph.filesz > ph.memsz {
            return Err(ElfError::BadProgramHeader);
        }
        if image.segment_count == MAX_LOAD_SEGMENTS {
            return Err(ElfError::TooManySegments);
        }

        let start = ph.offset as usize;
        let end = start + ph.filesz as usize;
        let data = bytes.get(start..end).ok_or(ElfError::SegmentOutOfRange)?;
        image.segments[image.segment_count] = Some(LoadSegment {
            vaddr: ph.vaddr as usize,
            file_size: ph.filesz as usize,
            mem_size: ph.memsz as usize,
            flags: ph.flags,
            data,
        });
        image.segment_count += 1;
    }

    if image.segment_count == 0 {
        return Err(ElfError::NoLoadSegments);
    }

    Ok(image)
}

pub fn plan_shared_object_load(
    bytes: &[u8],
    preferred_base: usize,
) -> Result<SharedObjectLoadPlan, ElfError> {
    let hdr = parse_elf_header(bytes)?;
    if hdr.file_type != ElfFileType::SharedObject {
        return Err(ElfError::UnsupportedType);
    }

    let image = load_user_elf(bytes, 0)?;
    Ok(SharedObjectLoadPlan {
        preferred_base,
        entry_offset: hdr.entry as usize,
        segment_count: image.segment_count,
        needs_relocation: true,
    })
}

pub fn load_init_from_vfs<'a>(
    vfs: &Vfs<'_>,
    path: &str,
    scratch: &'a mut [u8],
    stack_top: usize,
) -> Result<LoadedUserImage<'a>, InitLoadError> {
    let used = vfs.read_all(path, scratch)?;
    Ok(load_user_elf(&scratch[..used], stack_top)?)
}

fn le16(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([buf[offset], buf[offset + 1]])
}

fn le32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]])
}

fn le64(buf: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
        buf[offset + 4],
        buf[offset + 5],
        buf[offset + 6],
        buf[offset + 7],
    ])
}
