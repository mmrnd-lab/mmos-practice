// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::{
    scheduler::{SchedError, ScheduleDecision, Scheduler, TaskState},
    trap::{dispatch_trap_outcome, TrapDispatchResult, TrapFrame, TrapOutcome},
};

pub struct TrapRuntime<const MAX_TASKS: usize> {
    scheduler: Scheduler<MAX_TASKS>,
}

impl<const MAX_TASKS: usize> TrapRuntime<MAX_TASKS> {
    pub const fn new() -> Self {
        Self {
            scheduler: Scheduler::new(),
        }
    }

    pub fn add_runnable(&mut self, tid: usize, frame: TrapFrame) -> Result<(), SchedError> {
        self.scheduler.add_runnable(tid, frame)
    }

    pub fn start_first_task(&mut self) {
        let _ = self.scheduler.on_tick();
    }

    pub fn mark_blocked(&mut self, tid: usize) -> bool {
        self.scheduler.set_state(tid, TaskState::Blocked)
    }

    pub fn current_tid(&self) -> Option<usize> {
        self.scheduler.current_tid()
    }

    pub fn task_frame(&self, tid: usize) -> Option<TrapFrame> {
        self.scheduler.task_frame(tid)
    }

    pub fn dispatch_outcome(
        &mut self,
        frame: &mut TrapFrame,
        outcome: TrapOutcome,
    ) -> TrapDispatchResult {
        dispatch_trap_outcome(frame, &mut self.scheduler, outcome)
    }

    pub fn dispatch_schedule(&mut self, frame: &mut TrapFrame) -> TrapDispatchResult {
        self.dispatch_outcome(frame, TrapOutcome::Schedule)
    }

    pub fn scheduler(&self) -> &Scheduler<MAX_TASKS> {
        &self.scheduler
    }

    pub fn scheduler_mut(&mut self) -> &mut Scheduler<MAX_TASKS> {
        &mut self.scheduler
    }
}

pub fn switched_tids(result: TrapDispatchResult) -> Option<(usize, usize)> {
    match result {
        TrapDispatchResult::Switched(ScheduleDecision::Switched { prev_tid, next_tid }) => {
            Some((prev_tid, next_tid))
        }
        _ => None,
    }
}
