// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug)]
pub struct PageFrameAllocator {
    base: usize,
    frame_size: usize,
    next: usize,
    limit: usize,
}

impl PageFrameAllocator {
    pub fn new(base: usize, limit: usize, frame_size: usize) -> Self {
        Self {
            base,
            frame_size,
            next: base,
            limit,
        }
    }

    pub fn alloc(&mut self) -> Option<usize> {
        if self.next + self.frame_size > self.limit {
            return None;
        }
        let frame = self.next;
        self.next += self.frame_size;
        Some(frame)
    }

    pub fn reset(&mut self) {
        self.next = self.base;
    }
}
