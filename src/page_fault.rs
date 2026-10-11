// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::pmm::PageFrameManager;
use crate::sv39::{PAGE_SIZE, PTE_A, PTE_D, PTE_V};
use crate::vm::{FaultAccess, VmSpace};

pub const SCAUSE_INST_PAGE_FAULT: usize = 12;
pub const SCAUSE_LOAD_PAGE_FAULT: usize = 13;
pub const SCAUSE_STORE_PAGE_FAULT: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFaultMapping {
    pub va_page: usize,
    pub frame_pa: usize,
    pub pte_flags: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageFaultResult {
    Mapped(PageFaultMapping),
    InvalidAccess,
    OutOfMemory,
    Ignored,
}

pub fn handle_user_page_fault<const A: usize, const F: usize>(
    scause: usize,
    stval: usize,
    vm: &VmSpace<A>,
    pmm: &mut PageFrameManager<F>,
) -> PageFaultResult {
    let access = match scause {
        SCAUSE_INST_PAGE_FAULT => FaultAccess::Execute,
        SCAUSE_LOAD_PAGE_FAULT => FaultAccess::Read,
        SCAUSE_STORE_PAGE_FAULT => FaultAccess::Write,
        _ => return PageFaultResult::Ignored,
    };

    let mut flags = match vm.resolve_fault(stval, access) {
        Some(v) => v,
        None => return PageFaultResult::InvalidAccess,
    };
    flags |= PTE_V | PTE_A;
    if let FaultAccess::Write = access {
        flags |= PTE_D;
    }

    let frame_pa = match pmm.alloc() {
        Some(p) => p,
        None => return PageFaultResult::OutOfMemory,
    };

    PageFaultResult::Mapped(PageFaultMapping {
        va_page: stval & !(PAGE_SIZE - 1),
        frame_pa,
        pte_flags: flags,
    })
}
