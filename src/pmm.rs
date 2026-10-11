// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmmError {
    OutOfRange,
    Unaligned,
    FreeListFull,
}

#[derive(Debug)]
pub struct PageFrameManager<const FREE_CAP: usize> {
    base: usize,
    limit: usize,
    frame_size: usize,
    next: usize,
    free: [usize; FREE_CAP],
    free_len: usize,
}

impl<const FREE_CAP: usize> PageFrameManager<FREE_CAP> {
    pub fn new(base: usize, limit: usize, frame_size: usize) -> Self {
        Self {
            base,
            limit,
            frame_size,
            next: base,
            free: [0; FREE_CAP],
            free_len: 0,
        }
    }

    pub fn alloc(&mut self) -> Option<usize> {
        if self.free_len > 0 {
            self.free_len -= 1;
            return Some(self.free[self.free_len]);
        }
        if self.next + self.frame_size > self.limit {
            return None;
        }
        let frame = self.next;
        self.next += self.frame_size;
        Some(frame)
    }

    pub fn free(&mut self, frame: usize) -> Result<(), PmmError> {
        if frame < self.base || frame + self.frame_size > self.limit {
            return Err(PmmError::OutOfRange);
        }
        if frame % self.frame_size != 0 {
            return Err(PmmError::Unaligned);
        }
        if self.free_len >= FREE_CAP {
            return Err(PmmError::FreeListFull);
        }
        self.free[self.free_len] = frame;
        self.free_len += 1;
        Ok(())
    }
}
