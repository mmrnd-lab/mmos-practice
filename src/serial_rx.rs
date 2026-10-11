// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::scheduler::Scheduler;
use crate::wait_queue::WaitQueue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SerialRxEvent {
    bytes_available: usize,
}

impl SerialRxEvent {
    pub const fn new(bytes_available: usize) -> Self {
        Self { bytes_available }
    }

    pub const fn bytes_available(self) -> usize {
        self.bytes_available
    }

    pub const fn has_input(self) -> bool {
        self.bytes_available > 0
    }
}

pub fn wake_reader_on_rx_event<const MAX_WAITERS: usize, const MAX_TASKS: usize>(
    event: SerialRxEvent,
    waiters: &mut WaitQueue<MAX_WAITERS>,
    scheduler: &mut Scheduler<MAX_TASKS>,
) -> Option<usize> {
    if event.has_input() {
        waiters.wake_one(scheduler)
    } else {
        None
    }
}
