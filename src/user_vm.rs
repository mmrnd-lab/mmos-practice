// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::elf_loader::{pflags_to_pte, LoadedUserImage};
use crate::sv39::{
    make_satp, PageTableEntry, RootPageTable, PAGE_SIZE, PTE_A, PTE_D, PTE_G, PTE_R,
    PTE_U, PTE_V, PTE_W, PTE_X, PT_ENTRIES,
};

pub const GIB_SHIFT: usize = 30;
const PPN_GIB_SHIFT: usize = 18;

pub const KERNEL_IDENTITY_VPN2: usize = 2;
pub const KERNEL_IDENTITY_PPN: usize = KERNEL_IDENTITY_VPN2 << PPN_GIB_SHIFT;
pub const KERNEL_IDENTITY_FLAGS: u64 = PTE_R | PTE_W | PTE_X | PTE_A | PTE_D | PTE_G;

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
    let existing = root.entries[KERNEL_IDENTITY_VPN2];
    if existing.is_valid() && (existing.flags() & PTE_U) != 0 {
        return Err(KernelMapError::UserMappingOverlap {
            vpn2: KERNEL_IDENTITY_VPN2,
        });
    }
    root.map_gib_identity(
        KERNEL_IDENTITY_VPN2,
        KERNEL_IDENTITY_PPN,
        KERNEL_IDENTITY_FLAGS,
    );
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

pub fn is_kernel_mapping(entry: PageTableEntry) -> bool {
    let flags = entry.flags();
    entry.is_valid()
        && entry.ppn() == KERNEL_IDENTITY_PPN
        && (flags & PTE_U) == 0
        && (flags & PTE_G) != 0
        && (flags & (PTE_R | PTE_W | PTE_X)) == (PTE_R | PTE_W | PTE_X)
}
