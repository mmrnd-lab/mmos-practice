// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::sv39::{PTE_R, PTE_U, PTE_W, PTE_X};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmError {
    InvalidRange,
    TooManyAreas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultAccess {
    Read,
    Write,
    Execute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmArea {
    pub start: usize,
    pub end: usize,
    pub pte_flags: u64,
}

impl VmArea {
    fn contains(&self, addr: usize) -> bool {
        self.start <= addr && addr < self.end
    }

    fn allows(&self, access: FaultAccess) -> bool {
        match access {
            FaultAccess::Read => self.pte_flags & PTE_R != 0,
            FaultAccess::Write => self.pte_flags & PTE_W != 0,
            FaultAccess::Execute => self.pte_flags & PTE_X != 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmSpace<const MAX_AREAS: usize> {
    areas: [Option<VmArea>; MAX_AREAS],
    area_count: usize,
}

impl<const MAX_AREAS: usize> VmSpace<MAX_AREAS> {
    pub const fn new() -> Self {
        Self {
            areas: [None; MAX_AREAS],
            area_count: 0,
        }
    }

    pub fn map_anon(&mut self, start: usize, end: usize, pte_flags: u64) -> Result<(), VmError> {
        if start >= end {
            return Err(VmError::InvalidRange);
        }
        if self.area_count >= MAX_AREAS {
            return Err(VmError::TooManyAreas);
        }
        self.areas[self.area_count] = Some(VmArea {
            start,
            end,
            pte_flags: pte_flags | PTE_U,
        });
        self.area_count += 1;
        Ok(())
    }

    pub fn resolve_fault(&self, addr: usize, access: FaultAccess) -> Option<u64> {
        let mut idx = 0usize;
        while idx < self.area_count {
            if let Some(area) = self.areas[idx] {
                if area.contains(addr) && area.allows(access) {
                    return Some(area.pte_flags);
                }
            }
            idx += 1;
        }
        None
    }

    pub fn allows_range(&self, start: usize, len: usize, access: FaultAccess) -> bool {
        if len == 0 {
            return true;
        }
        let Some(end) = start.checked_add(len) else {
            return false;
        };
        let last = end - 1;
        self.resolve_fault(start, access).is_some() && self.resolve_fault(last, access).is_some()
    }
}
