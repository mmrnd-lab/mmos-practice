// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone, Copy)]
pub struct HeapRegion {
    pub start: usize,
    pub size: usize,
}

impl HeapRegion {
    pub fn end(&self) -> usize {
        self.start + self.size
    }
}

pub fn init_kheap(start: usize, size: usize) -> HeapRegion {
    HeapRegion { start, size }
}
