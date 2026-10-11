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
    MalformedImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameBackedMaterializeError {
    MissingSegmentFrame(usize),
    SegmentFrameTooSmall(usize),
    StackFrameTooSmall,
    MalformedImage,
}

#[derive(Debug)]
pub struct UserRegionMut<'a> {
    pub vaddr: usize,
    pub bytes: &'a mut [u8],
}

#[derive(Debug)]
pub struct FrameRegionMut<'a> {
    pub frame_pa: usize,
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
            let seg = match loaded.segments[idx] {
                Some(v) => v,
                None => {
                    out.segment_count = idx;
                    break;
                }
            };
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
        let seg = loaded.segments[seg_idx].ok_or(ExecError::MalformedImage)?;

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

pub fn materialize_frame_backed_user_image(
    loaded: &LoadedUserImage<'_>,
    plan: &crate::user_vm::UserFrameBackedImagePlan,
    regions: &mut [FrameRegionMut<'_>],
    stack_frame: &mut [u8],
    stack_bytes: &[u8],
) -> Result<(), FrameBackedMaterializeError> {
    let mut idx = 0usize;
    while idx < plan.segment_count {
        let frame_plan = plan.segments[idx].ok_or(FrameBackedMaterializeError::MalformedImage)?;
        let seg = loaded.segments[idx].ok_or(FrameBackedMaterializeError::MalformedImage)?;
        let region_idx = find_frame_region(regions, frame_plan.frame_pa).ok_or(
            FrameBackedMaterializeError::MissingSegmentFrame(frame_plan.frame_pa),
        )?;
        let page_offset = frame_plan.vaddr & (crate::sv39::PAGE_SIZE - 1);
        if regions[region_idx].bytes.len() < page_offset + frame_plan.mem_size {
            return Err(FrameBackedMaterializeError::SegmentFrameTooSmall(
                frame_plan.frame_pa,
            ));
        }

        if !frame_seen_before(plan, idx, frame_plan.frame_pa) {
            regions[region_idx].bytes.fill(0);
        }
        let dst = &mut regions[region_idx].bytes[page_offset..page_offset + frame_plan.mem_size];
        dst[..seg.file_size].copy_from_slice(seg.data);
        dst[seg.file_size..].fill(0);
        idx += 1;
    }

    if stack_frame.len() < stack_bytes.len() {
        return Err(FrameBackedMaterializeError::StackFrameTooSmall);
    }
    stack_frame.fill(0);
    stack_frame[..stack_bytes.len()].copy_from_slice(stack_bytes);
    Ok(())
}

#[cfg(target_arch = "riscv64")]
pub unsafe fn materialize_frame_backed_user_image_to_frames(
    loaded: &LoadedUserImage<'_>,
    plan: &crate::user_vm::UserFrameBackedImagePlan,
    stack_bytes: &[u8],
) -> Result<(), FrameBackedMaterializeError> {
    let mut idx = 0usize;
    while idx < plan.segment_count {
        let frame_plan = plan.segments[idx].ok_or(FrameBackedMaterializeError::MalformedImage)?;
        let seg = loaded.segments[idx].ok_or(FrameBackedMaterializeError::MalformedImage)?;
        let dst = unsafe {
            core::slice::from_raw_parts_mut(frame_plan.frame_pa as *mut u8, crate::sv39::PAGE_SIZE)
        };
        let page_offset = frame_plan.vaddr & (crate::sv39::PAGE_SIZE - 1);
        if dst.len() < page_offset + frame_plan.mem_size {
            return Err(FrameBackedMaterializeError::SegmentFrameTooSmall(
                frame_plan.frame_pa,
            ));
        }
        if !frame_seen_before(plan, idx, frame_plan.frame_pa) {
            dst.fill(0);
        }
        let dst = &mut dst[page_offset..page_offset + frame_plan.mem_size];
        dst[..seg.file_size].copy_from_slice(seg.data);
        idx += 1;
    }

    let stack = unsafe {
        core::slice::from_raw_parts_mut(
            plan.stack_frame_start_pa as *mut u8,
            crate::sv39::PAGE_SIZE,
        )
    };
    if stack.len() < stack_bytes.len() {
        return Err(FrameBackedMaterializeError::StackFrameTooSmall);
    }
    stack.fill(0);
    stack[..stack_bytes.len()].copy_from_slice(stack_bytes);
    Ok(())
}

fn frame_seen_before(
    plan: &crate::user_vm::UserFrameBackedImagePlan,
    current_idx: usize,
    frame_pa: usize,
) -> bool {
    let mut idx = 0usize;
    while idx < current_idx {
        if let Some(seg) = plan.segments[idx] {
            if seg.frame_pa == frame_pa {
                return true;
            }
        }
        idx += 1;
    }
    false
}

fn find_frame_region(regions: &[FrameRegionMut<'_>], frame_pa: usize) -> Option<usize> {
    let mut idx = 0usize;
    while idx < regions.len() {
        if regions[idx].frame_pa == frame_pa {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

pub fn load_init_exec_image<'a>(
    vfs: &Vfs<'_>,
    path: &str,
    scratch: &'a mut [u8],
    stack_top: usize,
) -> Result<LoadedUserImage<'a>, InitLoadError> {
    crate::elf_loader::load_init_from_vfs(vfs, path, scratch, stack_top)
}
