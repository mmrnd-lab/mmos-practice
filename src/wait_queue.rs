// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::scheduler::{Scheduler, TaskState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitQueueError {
    Full,
}

#[derive(Debug, Clone, Copy)]
pub struct WaitQueue<const MAX_WAITERS: usize> {
    tids: [Option<usize>; MAX_WAITERS],
}

impl<const MAX_WAITERS: usize> WaitQueue<MAX_WAITERS> {
    pub const fn new() -> Self {
        Self {
            tids: [None; MAX_WAITERS],
        }
    }

    pub fn contains(&self, tid: usize) -> bool {
        let mut idx = 0usize;
        while idx < MAX_WAITERS {
            if self.tids[idx] == Some(tid) {
                return true;
            }
            idx += 1;
        }
        false
    }

    pub fn enqueue(&mut self, tid: usize) -> Result<(), WaitQueueError> {
        if self.contains(tid) {
            return Ok(());
        }

        let mut idx = 0usize;
        while idx < MAX_WAITERS {
            if self.tids[idx].is_none() {
                self.tids[idx] = Some(tid);
                return Ok(());
            }
            idx += 1;
        }
        Err(WaitQueueError::Full)
    }

    pub fn wake_one<const MAX_TASKS: usize>(
        &mut self,
        scheduler: &mut Scheduler<MAX_TASKS>,
    ) -> Option<usize> {
        let mut idx = 0usize;
        while idx < MAX_WAITERS {
            if let Some(tid) = self.tids[idx] {
                self.tids[idx] = None;
                scheduler.set_state(tid, TaskState::Runnable);
                return Some(tid);
            }
            idx += 1;
        }
        None
    }
}
