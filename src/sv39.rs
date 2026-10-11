// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

pub const PAGE_SIZE: usize = 4096;
pub const PT_ENTRIES: usize = 512;

pub const PTE_V: u64 = 1 << 0;
pub const PTE_R: u64 = 1 << 1;
pub const PTE_W: u64 = 1 << 2;
pub const PTE_X: u64 = 1 << 3;
pub const PTE_U: u64 = 1 << 4;
pub const PTE_G: u64 = 1 << 5;
pub const PTE_A: u64 = 1 << 6;
pub const PTE_D: u64 = 1 << 7;

pub const SATP_MODE_SV39: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageTableEntry(pub u64);

impl PageTableEntry {
    pub const fn invalid() -> Self {
        Self(0)
    }

    pub fn leaf(ppn: usize, flags: u64) -> Self {
        let ppn_bits = (ppn as u64) << 10;
        Self(ppn_bits | flags | PTE_V)
    }

    pub fn ppn(&self) -> usize {
        ((self.0 >> 10) & ((1 << 44) - 1)) as usize
    }

    pub fn flags(&self) -> u64 {
        self.0 & 0x3ff
    }

    pub const fn is_valid(&self) -> bool {
        (self.0 & PTE_V) != 0
    }
}

#[repr(C, align(4096))]
#[derive(Debug)]
pub struct RootPageTable {
    pub entries: [PageTableEntry; PT_ENTRIES],
}

impl RootPageTable {
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::invalid(); PT_ENTRIES],
        }
    }

    pub fn map_gib_identity(&mut self, vpn2: usize, ppn: usize, flags: u64) {
        self.entries[vpn2] = PageTableEntry::leaf(ppn, flags);
    }

    pub fn physical_addr(&self) -> usize {
        self as *const Self as usize
    }

    pub fn ppn(&self) -> usize {
        self.physical_addr() / PAGE_SIZE
    }

    pub fn is_page_aligned(&self) -> bool {
        self.physical_addr() % PAGE_SIZE == 0
    }
}

pub fn make_satp(root_ppn: usize, asid: usize) -> usize {
    (SATP_MODE_SV39 << 60) | ((asid & 0xffff) << 44) | (root_ppn & ((1 << 44) - 1))
}
