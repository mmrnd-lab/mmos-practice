// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::elf_loader::{
    materialize_shared_object_relocations, pflags_to_pte, plan_shared_object_mappings, ElfError,
    LoadedUserImage, SharedObjectMappingPlan,
};
use crate::ext4::FsError;
use crate::sv39::{
    make_satp, PageTableEntry, RootPageTable, PAGE_SIZE, PTE_A, PTE_D, PTE_G, PTE_R,
    PTE_U, PTE_V, PTE_W, PTE_X, PT_ENTRIES,
};
use crate::vfs::Vfs;

pub const GIB_SHIFT: usize = 30;
const PPN_GIB_SHIFT: usize = 18;

pub const KERNEL_IDENTITY_VPN2: usize = 2;
pub const KERNEL_IDENTITY_PPN: usize = KERNEL_IDENTITY_VPN2 << PPN_GIB_SHIFT;
pub const KERNEL_IDENTITY_FLAGS: u64 = PTE_R | PTE_W | PTE_X | PTE_A | PTE_D | PTE_G;
pub const KERNEL_MMIO_VPN2: usize = 0;
pub const KERNEL_MMIO_PPN: usize = KERNEL_MMIO_VPN2 << PPN_GIB_SHIFT;
pub const KERNEL_MMIO_FLAGS: u64 = PTE_R | PTE_W | PTE_A | PTE_D | PTE_G;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserVmError {
    NoSegments,
    TooManyRegions,
    MalformedImage,
    UnalignedRootFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelMapError {
    UserMappingOverlap { vpn2: usize },
}

#[derive(Debug)]
pub struct UserVmPlan {
    pub root_frame_pa: usize,
    pub satp: usize,
    pub mapped_regions: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserVmRootInstall {
    pub root_frame_pa: usize,
    pub root_ppn: usize,
    pub satp: usize,
    pub mapped_regions: usize,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserSegmentFramePlan {
    pub vaddr: usize,
    pub frame_pa: usize,
    pub file_size: usize,
    pub mem_size: usize,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserFrameBackedImagePlan {
    pub entry_vaddr: usize,
    pub entry_frame_pa: usize,
    pub stack_top: usize,
    pub stack_frame_start_pa: usize,
    pub stack_pages: usize,
    pub segment_count: usize,
    pub segments: [Option<UserSegmentFramePlan>; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedObjectFramePlan {
    pub vaddr: usize,
    pub frame_pa: usize,
    pub file_size: usize,
    pub mem_size: usize,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedObjectFrameBackedPlan {
    pub base: usize,
    pub entry_vaddr: usize,
    pub entry_frame_pa: usize,
    pub segment_count: usize,
    pub segments: [Option<SharedObjectFramePlan>; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedObjectLoadPipeline {
    pub relocation_count: usize,
    pub base: usize,
    pub entry_vaddr: usize,
    pub entry_frame_pa: usize,
    pub segment_count: usize,
    pub mapped_pages: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserFrameBackedImageError {
    NoSegments,
    TooManySegments,
    SegmentTooLarge,
    StackTooLarge,
    OutOfFrames,
    EntryNotMapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedObjectFramePlanError {
    NoSegments,
    TooManySegments,
    SegmentTooLarge,
    OutOfFrames,
    EntryNotMapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedObjectLoadPipelineError {
    Elf(ElfError),
    Frame(SharedObjectFramePlanError),
    Mapping(UserFrameBackedMappingError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedObjectPathLoadError {
    Fs(FsError),
    Pipeline(SharedObjectLoadPipelineError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserFrameBackedMappingInstall {
    pub l1_frame_pa: usize,
    pub l0_frame_pa: usize,
    pub mapped_pages: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserFrameBackedMappingError {
    MissingSegment,
    UnalignedTableFrame,
    UnsupportedAddressRange,
    OutOfFrames,
}

#[derive(Debug)]
pub struct ProcessAddressSpace {
    pub root_frame_pa: usize,
    pub satp: usize,
    pub mapped_regions: usize,
}

impl ProcessAddressSpace {
    pub const fn empty() -> Self {
        Self {
            root_frame_pa: 0,
            satp: 0,
            mapped_regions: 0,
        }
    }

    pub fn install_user_image(
        &mut self,
        loaded: &LoadedUserImage<'_>,
        root_frame_pa: usize,
        root: &mut RootPageTable,
    ) -> Result<UserVmRootInstall, UserVmError> {
        let install = install_user_vm_plan_in_root(loaded, root_frame_pa, root)?;
        self.root_frame_pa = install.root_frame_pa;
        self.satp = install.satp;
        self.mapped_regions = install.mapped_regions;
        Ok(install)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSpaceTableError {
    TableFull,
    MissingAddressSpace,
}

#[derive(Debug)]
pub struct ProcessAddressSpaceTable<const MAX_SPACES: usize> {
    slots: [Option<(usize, ProcessAddressSpace)>; MAX_SPACES],
    next_id: usize,
}

impl<const MAX_SPACES: usize> ProcessAddressSpaceTable<MAX_SPACES> {
    pub const fn new() -> Self {
        Self {
            slots: [const { None }; MAX_SPACES],
            next_id: 1,
        }
    }

    pub fn create_empty(&mut self) -> Result<usize, AddressSpaceTableError> {
        let mut idx = 0usize;
        while idx < MAX_SPACES {
            if self.slots[idx].is_none() {
                let id = self.next_id;
                self.next_id = self.next_id.wrapping_add(1);
                self.slots[idx] = Some((id, ProcessAddressSpace::empty()));
                return Ok(id);
            }
            idx += 1;
        }
        Err(AddressSpaceTableError::TableFull)
    }

    pub fn get(&self, id: usize) -> Result<&ProcessAddressSpace, AddressSpaceTableError> {
        self.find_index(id)
            .and_then(|idx| self.slots[idx].as_ref().map(|(_, space)| space))
            .ok_or(AddressSpaceTableError::MissingAddressSpace)
    }

    pub fn get_mut(
        &mut self,
        id: usize,
    ) -> Result<&mut ProcessAddressSpace, AddressSpaceTableError> {
        self.find_index(id)
            .and_then(|idx| self.slots[idx].as_mut().map(|(_, space)| space))
            .ok_or(AddressSpaceTableError::MissingAddressSpace)
    }

    pub fn satp(&self, id: usize) -> Result<usize, AddressSpaceTableError> {
        Ok(self.get(id)?.satp)
    }

    pub fn install_user_image(
        &mut self,
        id: usize,
        loaded: &LoadedUserImage<'_>,
        root_frame_pa: usize,
        root: &mut RootPageTable,
    ) -> Result<UserVmRootInstall, UserVmError> {
        self.get_mut(id)
            .map_err(|_| UserVmError::MalformedImage)?
            .install_user_image(loaded, root_frame_pa, root)
    }

    fn find_index(&self, id: usize) -> Option<usize> {
        let mut idx = 0usize;
        while idx < MAX_SPACES {
            if let Some((slot_id, _)) = self.slots[idx].as_ref() {
                if *slot_id == id {
                    return Some(idx);
                }
            }
            idx += 1;
        }
        None
    }
}

pub fn plan_frame_backed_user_image<const FREE_CAP: usize>(
    loaded: &LoadedUserImage<'_>,
    pmm: &mut crate::pmm::PageFrameManager<FREE_CAP>,
    stack_bytes_len: usize,
) -> Result<UserFrameBackedImagePlan, UserFrameBackedImageError> {
    if loaded.segment_count == 0 {
        return Err(UserFrameBackedImageError::NoSegments);
    }
    if loaded.segment_count > 4 {
        return Err(UserFrameBackedImageError::TooManySegments);
    }
    if stack_bytes_len > PAGE_SIZE {
        return Err(UserFrameBackedImageError::StackTooLarge);
    }

    let mut segments: [Option<UserSegmentFramePlan>; 4] = [None; 4];
    let mut entry_frame_pa = 0usize;
    let mut seg_idx = 0usize;
    while seg_idx < loaded.segment_count {
        let seg = loaded.segments[seg_idx].ok_or(UserFrameBackedImageError::NoSegments)?;
        let page_offset = seg.vaddr & (PAGE_SIZE - 1);
        if page_offset + seg.mem_size > PAGE_SIZE {
            return Err(UserFrameBackedImageError::SegmentTooLarge);
        }
        let page_vaddr = seg.vaddr & !(PAGE_SIZE - 1);
        let mut frame_pa = 0usize;
        let mut prev = 0usize;
        while prev < seg_idx {
            if let Some(prev_seg) = segments[prev] {
                if (prev_seg.vaddr & !(PAGE_SIZE - 1)) == page_vaddr {
                    frame_pa = prev_seg.frame_pa;
                    break;
                }
            }
            prev += 1;
        }
        if frame_pa == 0 {
            frame_pa = pmm.alloc().ok_or(UserFrameBackedImageError::OutOfFrames)?;
        }
        if loaded.entry >= seg.vaddr && loaded.entry < seg.vaddr + seg.mem_size {
            entry_frame_pa = frame_pa + (loaded.entry - page_vaddr);
        }
        segments[seg_idx] = Some(UserSegmentFramePlan {
            vaddr: seg.vaddr,
            frame_pa,
            file_size: seg.file_size,
            mem_size: seg.mem_size,
            flags: seg.flags,
        });
        seg_idx += 1;
    }

    if entry_frame_pa == 0 {
        return Err(UserFrameBackedImageError::EntryNotMapped);
    }

    let stack_pages = if stack_bytes_len == 0 {
        1
    } else {
        (stack_bytes_len + PAGE_SIZE - 1) / PAGE_SIZE
    };
    if stack_pages != 1 {
        return Err(UserFrameBackedImageError::StackTooLarge);
    }
    let stack_frame_start_pa = pmm.alloc().ok_or(UserFrameBackedImageError::OutOfFrames)?;

    Ok(UserFrameBackedImagePlan {
        entry_vaddr: loaded.entry,
        entry_frame_pa,
        stack_top: loaded.stack_top,
        stack_frame_start_pa,
        stack_pages,
        segment_count: loaded.segment_count,
        segments,
    })
}

pub fn plan_frame_backed_shared_object<const FREE_CAP: usize>(
    mapping: &SharedObjectMappingPlan,
    pmm: &mut crate::pmm::PageFrameManager<FREE_CAP>,
) -> Result<SharedObjectFrameBackedPlan, SharedObjectFramePlanError> {
    if mapping.segment_count == 0 {
        return Err(SharedObjectFramePlanError::NoSegments);
    }
    if mapping.segment_count > 4 {
        return Err(SharedObjectFramePlanError::TooManySegments);
    }

    let mut segments: [Option<SharedObjectFramePlan>; 4] = [None; 4];
    let mut entry_frame_pa = 0usize;
    let mut seg_idx = 0usize;
    while seg_idx < mapping.segment_count {
        let seg = mapping.segments[seg_idx].ok_or(SharedObjectFramePlanError::NoSegments)?;
        let page_offset = seg.vaddr & (PAGE_SIZE - 1);
        if page_offset + seg.mem_size > PAGE_SIZE {
            return Err(SharedObjectFramePlanError::SegmentTooLarge);
        }

        let page_vaddr = seg.vaddr & !(PAGE_SIZE - 1);
        let mut frame_pa = 0usize;
        let mut prev = 0usize;
        while prev < seg_idx {
            if let Some(prev_seg) = segments[prev] {
                if (prev_seg.vaddr & !(PAGE_SIZE - 1)) == page_vaddr {
                    frame_pa = prev_seg.frame_pa;
                    break;
                }
            }
            prev += 1;
        }
        if frame_pa == 0 {
            frame_pa = pmm.alloc().ok_or(SharedObjectFramePlanError::OutOfFrames)?;
        }
        if mapping.entry_vaddr >= seg.vaddr && mapping.entry_vaddr < seg.vaddr + seg.mem_size {
            entry_frame_pa = frame_pa + (mapping.entry_vaddr - page_vaddr);
        }
        segments[seg_idx] = Some(SharedObjectFramePlan {
            vaddr: seg.vaddr,
            frame_pa,
            file_size: seg.file_size,
            mem_size: seg.mem_size,
            flags: seg.flags,
        });
        seg_idx += 1;
    }

    if entry_frame_pa == 0 {
        return Err(SharedObjectFramePlanError::EntryNotMapped);
    }

    Ok(SharedObjectFrameBackedPlan {
        base: mapping.base,
        entry_vaddr: mapping.entry_vaddr,
        entry_frame_pa,
        segment_count: mapping.segment_count,
        segments,
    })
}

pub fn install_frame_backed_user_mapping_in_tables(
    root: &mut RootPageTable,
    l1: &mut RootPageTable,
    l1_frame_pa: usize,
    l0: &mut RootPageTable,
    l0_frame_pa: usize,
    plan: &UserFrameBackedImagePlan,
) -> Result<UserFrameBackedMappingInstall, UserFrameBackedMappingError> {
    if l1_frame_pa % PAGE_SIZE != 0 || l0_frame_pa % PAGE_SIZE != 0 {
        return Err(UserFrameBackedMappingError::UnalignedTableFrame);
    }

    clear_root_page_table(l1);
    clear_root_page_table(l0);

    let root_vpn2 = vpn2(plan.entry_vaddr);
    let l1_vpn1 = vpn1(plan.entry_vaddr);
    root.entries[root_vpn2] = PageTableEntry::leaf(l1_frame_pa / PAGE_SIZE, 0);
    l1.entries[l1_vpn1] = PageTableEntry::leaf(l0_frame_pa / PAGE_SIZE, 0);

    let mut mapped_pages = 0usize;
    let mut idx = 0usize;
    while idx < plan.segment_count {
        let seg = plan.segments[idx].ok_or(UserFrameBackedMappingError::MissingSegment)?;
        ensure_same_leaf_table(plan.entry_vaddr, seg.vaddr)?;
        let idx0 = vpn0(seg.vaddr);
        let flags = pflags_to_pte(seg.flags) | PTE_A | PTE_D;
        let existing = l0.entries[idx0];
        if existing.is_valid() {
            if existing.ppn() != seg.frame_pa / PAGE_SIZE {
                return Err(UserFrameBackedMappingError::UnsupportedAddressRange);
            }
            l0.entries[idx0] = PageTableEntry::leaf(existing.ppn(), existing.flags() | flags);
        } else {
            l0.entries[idx0] = PageTableEntry::leaf(seg.frame_pa / PAGE_SIZE, flags);
            mapped_pages += 1;
        }
        idx += 1;
    }

    let stack_vaddr = plan.stack_top - PAGE_SIZE;
    ensure_same_leaf_table(plan.entry_vaddr, stack_vaddr)?;
    l0.entries[vpn0(stack_vaddr)] = PageTableEntry::leaf(
        plan.stack_frame_start_pa / PAGE_SIZE,
        PTE_U | PTE_R | PTE_W | PTE_A | PTE_D,
    );
    mapped_pages += plan.stack_pages;

    Ok(UserFrameBackedMappingInstall {
        l1_frame_pa,
        l0_frame_pa,
        mapped_pages,
    })
}

pub fn install_frame_backed_shared_object_mapping_in_tables(
    root: &mut RootPageTable,
    l1: &mut RootPageTable,
    l1_frame_pa: usize,
    l0: &mut RootPageTable,
    l0_frame_pa: usize,
    plan: &SharedObjectFrameBackedPlan,
) -> Result<UserFrameBackedMappingInstall, UserFrameBackedMappingError> {
    if l1_frame_pa % PAGE_SIZE != 0 || l0_frame_pa % PAGE_SIZE != 0 {
        return Err(UserFrameBackedMappingError::UnalignedTableFrame);
    }

    clear_root_page_table(l1);
    clear_root_page_table(l0);

    let root_vpn2 = vpn2(plan.entry_vaddr);
    let l1_vpn1 = vpn1(plan.entry_vaddr);
    root.entries[root_vpn2] = PageTableEntry::leaf(l1_frame_pa / PAGE_SIZE, 0);
    l1.entries[l1_vpn1] = PageTableEntry::leaf(l0_frame_pa / PAGE_SIZE, 0);

    let mut mapped_pages = 0usize;
    let mut idx = 0usize;
    while idx < plan.segment_count {
        let seg = plan.segments[idx].ok_or(UserFrameBackedMappingError::MissingSegment)?;
        ensure_same_leaf_table(plan.entry_vaddr, seg.vaddr)?;
        let idx0 = vpn0(seg.vaddr);
        let flags = pflags_to_pte(seg.flags) | PTE_A | PTE_D;
        let existing = l0.entries[idx0];
        if existing.is_valid() {
            if existing.ppn() != seg.frame_pa / PAGE_SIZE {
                return Err(UserFrameBackedMappingError::UnsupportedAddressRange);
            }
            l0.entries[idx0] = PageTableEntry::leaf(existing.ppn(), existing.flags() | flags);
        } else {
            l0.entries[idx0] = PageTableEntry::leaf(seg.frame_pa / PAGE_SIZE, flags);
            mapped_pages += 1;
        }
        idx += 1;
    }

    Ok(UserFrameBackedMappingInstall {
        l1_frame_pa,
        l0_frame_pa,
        mapped_pages,
    })
}

pub fn load_shared_object_pipeline_in_tables<const FREE_CAP: usize>(
    image: &mut [u8],
    preferred_base: usize,
    pmm: &mut crate::pmm::PageFrameManager<FREE_CAP>,
    root: &mut RootPageTable,
    l1: &mut RootPageTable,
    l1_frame_pa: usize,
    l0: &mut RootPageTable,
    l0_frame_pa: usize,
) -> Result<SharedObjectLoadPipeline, SharedObjectLoadPipelineError> {
    let relocation_count = materialize_shared_object_relocations(image, preferred_base)
        .map_err(SharedObjectLoadPipelineError::Elf)?;
    let mapping = plan_shared_object_mappings(image, preferred_base)
        .map_err(SharedObjectLoadPipelineError::Elf)?;
    let frame_plan = plan_frame_backed_shared_object(&mapping, pmm)
        .map_err(SharedObjectLoadPipelineError::Frame)?;
    let install = install_frame_backed_shared_object_mapping_in_tables(
        root,
        l1,
        l1_frame_pa,
        l0,
        l0_frame_pa,
        &frame_plan,
    )
    .map_err(SharedObjectLoadPipelineError::Mapping)?;

    Ok(SharedObjectLoadPipeline {
        relocation_count,
        base: frame_plan.base,
        entry_vaddr: frame_plan.entry_vaddr,
        entry_frame_pa: frame_plan.entry_frame_pa,
        segment_count: frame_plan.segment_count,
        mapped_pages: install.mapped_pages,
    })
}

pub fn load_shared_object_from_vfs_in_tables<const FREE_CAP: usize>(
    vfs: &Vfs<'_>,
    path: &str,
    scratch: &mut [u8],
    preferred_base: usize,
    pmm: &mut crate::pmm::PageFrameManager<FREE_CAP>,
    root: &mut RootPageTable,
    l1: &mut RootPageTable,
    l1_frame_pa: usize,
    l0: &mut RootPageTable,
    l0_frame_pa: usize,
) -> Result<SharedObjectLoadPipeline, SharedObjectPathLoadError> {
    let used = vfs.read_all(path, scratch).map_err(SharedObjectPathLoadError::Fs)?;
    load_shared_object_pipeline_in_tables(
        &mut scratch[..used],
        preferred_base,
        pmm,
        root,
        l1,
        l1_frame_pa,
        l0,
        l0_frame_pa,
    )
    .map_err(SharedObjectPathLoadError::Pipeline)
}

pub fn install_frame_backed_user_mapping<const FREE_CAP: usize>(
    root: &mut RootPageTable,
    plan: &UserFrameBackedImagePlan,
    pmm: &mut crate::pmm::PageFrameManager<FREE_CAP>,
) -> Result<UserFrameBackedMappingInstall, UserFrameBackedMappingError> {
    let l1_frame_pa = pmm.alloc().ok_or(UserFrameBackedMappingError::OutOfFrames)?;
    let l0_frame_pa = pmm.alloc().ok_or(UserFrameBackedMappingError::OutOfFrames)?;
    let l1 = unsafe { root_page_table_from_identity_frame_mut(l1_frame_pa) };
    let l0 = unsafe { root_page_table_from_identity_frame_mut(l0_frame_pa) };
    install_frame_backed_user_mapping_in_tables(root, l1, l1_frame_pa, l0, l0_frame_pa, plan)
}

fn ensure_same_leaf_table(base: usize, vaddr: usize) -> Result<(), UserFrameBackedMappingError> {
    if vpn2(base) != vpn2(vaddr) || vpn1(base) != vpn1(vaddr) {
        return Err(UserFrameBackedMappingError::UnsupportedAddressRange);
    }
    Ok(())
}

pub fn vpn2(vaddr: usize) -> usize {
    (vaddr >> 30) & 0x1ff
}

pub fn vpn1(vaddr: usize) -> usize {
    (vaddr >> 21) & 0x1ff
}

pub fn vpn0(vaddr: usize) -> usize {
    (vaddr >> 12) & 0x1ff
}

pub fn is_table_mapping(entry: PageTableEntry) -> bool {
    entry.is_valid() && (entry.flags() & (PTE_R | PTE_W | PTE_X)) == 0
}

pub fn build_user_vm_plan(
    loaded: &LoadedUserImage<'_>,
    root_frame_pa: usize,
) -> Result<UserVmPlan, UserVmError> {
    if loaded.segment_count == 0 {
        return Err(UserVmError::NoSegments);
    }
    if root_frame_pa % PAGE_SIZE != 0 {
        return Err(UserVmError::UnalignedRootFrame);
    }

    // Keep this stage simple: map 1GiB identity leaves for every vpn2 touched
    // by ELF segments and the initial user stack.
    let mut pairs = [(0usize, 0u64); 8];
    let mut count = 0usize;

    let mut seg_idx = 0usize;
    while seg_idx < loaded.segment_count {
        let seg = loaded.segments[seg_idx].ok_or(UserVmError::MalformedImage)?;
        let start = seg.vaddr;
        let end = seg.vaddr + seg.mem_size.saturating_sub(1);
        let flags = pflags_to_pte(seg.flags) | PTE_A | PTE_D;
        add_region_range(&mut pairs, &mut count, start, end, flags)?;
        seg_idx += 1;
    }

    let stack_addr = loaded.stack_top.saturating_sub(1);
    add_region(&mut pairs, &mut count, stack_addr >> GIB_SHIFT, PTE_U | crate::sv39::PTE_R | crate::sv39::PTE_W | PTE_A | PTE_D)?;

    Ok(UserVmPlan {
        root_frame_pa,
        satp: make_satp(root_frame_pa / PAGE_SIZE, 0),
        mapped_regions: count,
    })
}

pub fn install_user_vm_plan_in_root(
    loaded: &LoadedUserImage<'_>,
    root_frame_pa: usize,
    root: &mut RootPageTable,
) -> Result<UserVmRootInstall, UserVmError> {
    if root_frame_pa % PAGE_SIZE != 0 {
        return Err(UserVmError::UnalignedRootFrame);
    }

    let plan = build_user_vm_plan(loaded, root_frame_pa)?;
    clear_root_page_table(root);
    install_user_mappings(loaded, root)?;

    Ok(UserVmRootInstall {
        root_frame_pa,
        root_ppn: root_frame_pa / PAGE_SIZE,
        satp: plan.satp,
        mapped_regions: plan.mapped_regions,
    })
}

pub unsafe fn root_page_table_from_identity_frame_mut<'a>(
    root_frame_pa: usize,
) -> &'a mut RootPageTable {
    unsafe { &mut *(root_frame_pa as *mut RootPageTable) }
}

fn clear_root_page_table(root: &mut RootPageTable) {
    let mut idx = 0usize;
    while idx < PT_ENTRIES {
        root.entries[idx] = PageTableEntry::invalid();
        idx += 1;
    }
}

fn install_user_mappings(
    loaded: &LoadedUserImage<'_>,
    root: &mut RootPageTable,
) -> Result<usize, UserVmError> {
    let mut pairs = [(0usize, 0u64); 8];
    let mut count = 0usize;

    let mut seg_idx = 0usize;
    while seg_idx < loaded.segment_count {
        let seg = loaded.segments[seg_idx].ok_or(UserVmError::MalformedImage)?;
        let start = seg.vaddr;
        let end = seg.vaddr + seg.mem_size.saturating_sub(1);
        let flags = pflags_to_pte(seg.flags) | PTE_A | PTE_D;
        add_region_range(&mut pairs, &mut count, start, end, flags)?;
        seg_idx += 1;
    }

    let stack_addr = loaded.stack_top.saturating_sub(1);
    add_region(
        &mut pairs,
        &mut count,
        stack_addr >> GIB_SHIFT,
        PTE_U | crate::sv39::PTE_R | crate::sv39::PTE_W | PTE_A | PTE_D,
    )?;

    let mut i = 0usize;
    while i < count {
        let (vpn2, flags) = pairs[i];
        if vpn2 < PT_ENTRIES {
            let ppn = vpn2 << PPN_GIB_SHIFT;
            root.map_gib_identity(vpn2, ppn, flags);
        }
        i += 1;
    }

    Ok(count)
}

fn add_region_range(
    pairs: &mut [(usize, u64)],
    count: &mut usize,
    start_addr: usize,
    end_addr: usize,
    flags: u64,
) -> Result<(), UserVmError> {
    let start = start_addr >> GIB_SHIFT;
    let end = end_addr >> GIB_SHIFT;
    let mut vpn2 = start;
    while vpn2 <= end {
        add_region(pairs, count, vpn2, flags)?;
        vpn2 = vpn2.saturating_add(1);
        if vpn2 == 0 {
            break;
        }
    }
    Ok(())
}

fn add_region(
    pairs: &mut [(usize, u64)],
    count: &mut usize,
    vpn2: usize,
    flags: u64,
) -> Result<(), UserVmError> {
    let mut i = 0usize;
    while i < *count {
        if pairs[i].0 == vpn2 {
            pairs[i].1 |= flags;
            return Ok(());
        }
        i += 1;
    }
    if *count >= pairs.len() {
        return Err(UserVmError::TooManyRegions);
    }
    pairs[*count] = (vpn2, flags);
    *count += 1;
    Ok(())
}

pub fn is_leaf_user_mapping(entry: crate::sv39::PageTableEntry) -> bool {
    let f = entry.flags();
    (f & (PTE_V | PTE_U)) == (PTE_V | PTE_U)
}


pub fn install_kernel_identity_mapping(root: &mut RootPageTable) -> Result<(), KernelMapError> {
    install_supervisor_identity_leaf(root, KERNEL_IDENTITY_VPN2, KERNEL_IDENTITY_PPN, KERNEL_IDENTITY_FLAGS)?;
    install_supervisor_identity_leaf(root, KERNEL_MMIO_VPN2, KERNEL_MMIO_PPN, KERNEL_MMIO_FLAGS)?;
    Ok(())
}

fn install_supervisor_identity_leaf(
    root: &mut RootPageTable,
    vpn2: usize,
    ppn: usize,
    flags: u64,
) -> Result<(), KernelMapError> {
    let existing = root.entries[vpn2];
    if existing.is_valid() && (existing.flags() & PTE_U) != 0 {
        return Err(KernelMapError::UserMappingOverlap { vpn2 });
    }
    root.map_gib_identity(vpn2, ppn, flags);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SatpCommitPreflight {
    pub user_entry_vpn2: usize,
    pub user_stack_vpn2: usize,
    pub kernel_vpn2: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SatpCommitPreflightError {
    MissingUserEntry { vpn2: usize },
    MissingUserStack { vpn2: usize },
    MissingKernelMapping { vpn2: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelRuntimeSatpGuard {
    pub kernel_code_vpn2: usize,
    pub kernel_stack_vpn2: usize,
    pub trap_entry_vpn2: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelRuntimeSatpGuardError {
    MissingKernelCode { vpn2: usize },
    MissingKernelStack { vpn2: usize },
    MissingTrapEntry { vpn2: usize },
}

pub fn preflight_satp_commit(
    root: &RootPageTable,
    user_entry: usize,
    user_stack_top: usize,
) -> Result<SatpCommitPreflight, SatpCommitPreflightError> {
    let entry_vpn2 = user_entry >> GIB_SHIFT;
    let stack_vpn2 = user_stack_top.saturating_sub(1) >> GIB_SHIFT;

    if entry_vpn2 >= PT_ENTRIES || !is_leaf_user_mapping(root.entries[entry_vpn2]) {
        return Err(SatpCommitPreflightError::MissingUserEntry { vpn2: entry_vpn2 });
    }
    if stack_vpn2 >= PT_ENTRIES || !is_leaf_user_mapping(root.entries[stack_vpn2]) {
        return Err(SatpCommitPreflightError::MissingUserStack { vpn2: stack_vpn2 });
    }
    if !is_kernel_mapping(root.entries[KERNEL_IDENTITY_VPN2]) {
        return Err(SatpCommitPreflightError::MissingKernelMapping {
            vpn2: KERNEL_IDENTITY_VPN2,
        });
    }

    Ok(SatpCommitPreflight {
        user_entry_vpn2: entry_vpn2,
        user_stack_vpn2: stack_vpn2,
        kernel_vpn2: KERNEL_IDENTITY_VPN2,
    })
}

pub fn guard_kernel_runtime_after_satp(
    root: &RootPageTable,
    kernel_code_addr: usize,
    kernel_stack_addr: usize,
    trap_entry_addr: usize,
) -> Result<KernelRuntimeSatpGuard, KernelRuntimeSatpGuardError> {
    let kernel_code_vpn2 = kernel_code_addr >> GIB_SHIFT;
    let kernel_stack_vpn2 = kernel_stack_addr >> GIB_SHIFT;
    let trap_entry_vpn2 = trap_entry_addr >> GIB_SHIFT;

    if !kernel_mapping_covers(root, kernel_code_vpn2) {
        return Err(KernelRuntimeSatpGuardError::MissingKernelCode {
            vpn2: kernel_code_vpn2,
        });
    }
    if !kernel_mapping_covers(root, kernel_stack_vpn2) {
        return Err(KernelRuntimeSatpGuardError::MissingKernelStack {
            vpn2: kernel_stack_vpn2,
        });
    }
    if !kernel_mapping_covers(root, trap_entry_vpn2) {
        return Err(KernelRuntimeSatpGuardError::MissingTrapEntry {
            vpn2: trap_entry_vpn2,
        });
    }

    Ok(KernelRuntimeSatpGuard {
        kernel_code_vpn2,
        kernel_stack_vpn2,
        trap_entry_vpn2,
    })
}

fn kernel_mapping_covers(root: &RootPageTable, vpn2: usize) -> bool {
    vpn2 < PT_ENTRIES && is_kernel_mapping(root.entries[vpn2])
}

pub fn is_kernel_mapping(entry: PageTableEntry) -> bool {
    let flags = entry.flags();
    entry.is_valid()
        && entry.ppn() == KERNEL_IDENTITY_PPN
        && (flags & PTE_U) == 0
        && (flags & PTE_G) != 0
        && (flags & (PTE_R | PTE_W | PTE_X)) == (PTE_R | PTE_W | PTE_X)
}
