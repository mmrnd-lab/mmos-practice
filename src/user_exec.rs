// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::elf_loader::{InitLoadError, LoadedUserImage};
use crate::trap::TrapFrame;
use crate::user_init::{SSTATUS_SPIE, SSTATUS_SPP};
use crate::vfs::Vfs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecError {
    MissingRegion(usize),
    RegionTooSmall(usize),
}

#[derive(Debug)]
pub struct UserRegionMut<'a> {
    pub vaddr: usize,
    pub bytes: &'a mut [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentPlacement {
    pub vaddr: usize,
    pub file_size: usize,
    pub mem_size: usize,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserExecImage {
    pub entry: usize,
    pub stack_top: usize,
    pub segment_count: usize,
    pub segments: [Option<SegmentPlacement>; 4],
}

impl UserExecImage {
    pub fn from_loaded(loaded: &LoadedUserImage<'_>) -> Self {
        let mut out = Self {
            entry: loaded.entry,
            stack_top: loaded.stack_top,
            segment_count: loaded.segment_count,
            segments: [None; 4],
        };

        let mut idx = 0usize;
        while idx < loaded.segment_count {
            let seg = loaded.segments[idx].expect("segment slot must exist");
            out.segments[idx] = Some(SegmentPlacement {
                vaddr: seg.vaddr,
                file_size: seg.file_size,
                mem_size: seg.mem_size,
                flags: seg.flags,
            });
            idx += 1;
        }
        out
    }
}

pub fn install_exec_context(frame: &mut TrapFrame, image: &UserExecImage) {
    frame.sepc = image.entry;
    frame.set_sp(image.stack_top);
    // Ensure next return targets U-mode while leaving interrupt-enable semantics explicit.
    frame.sstatus &= !SSTATUS_SPP;
    frame.sstatus |= SSTATUS_SPIE;
}

pub fn materialize_segments(
    loaded: &LoadedUserImage<'_>,
    regions: &mut [UserRegionMut<'_>],
) -> Result<(), ExecError> {
    let mut seg_idx = 0usize;
    while seg_idx < loaded.segment_count {
        let seg = loaded.segments[seg_idx].expect("segment slot must exist");

        let mut found = None;
        let mut ridx = 0usize;
        while ridx < regions.len() {
            if regions[ridx].vaddr == seg.vaddr {
                found = Some(ridx);
                break;
            }
            ridx += 1;
        }

        let region_idx = found.ok_or(ExecError::MissingRegion(seg.vaddr))?;
        let region = &mut regions[region_idx];
        if region.bytes.len() < seg.mem_size {
            return Err(ExecError::RegionTooSmall(seg.vaddr));
        }

        let dst = &mut region.bytes[..seg.mem_size];
        let file_end = seg.file_size;
        dst[..file_end].copy_from_slice(seg.data);
        dst[file_end..].fill(0);

        seg_idx += 1;
    }

    Ok(())
}

pub fn load_init_exec_image<'a>(
    vfs: &Vfs<'_>,
    path: &str,
    scratch: &'a mut [u8],
    stack_top: usize,
) -> Result<LoadedUserImage<'a>, InitLoadError> {
    crate::elf_loader::load_init_from_vfs(vfs, path, scratch, stack_top)
}
