// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use core::cell::UnsafeCell;

use crate::{
    irq_guard::IrqGuard,
    process_model::{ProcError, ProcessTable, TaskControlBlock, TaskTable},
    scheduler::{SchedError, ScheduleDecision, Scheduler, TaskState},
    trap::{
        dispatch_exit_outcome_with_process_runtime, dispatch_trap_outcome, TrapDispatchResult,
        TrapFrame, TrapOutcome,
    },
    vm::VmSpace,
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

pub struct ProcessTrapRuntime<
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
> {
    processes: ProcessTable<MAX_PROCS, MAX_CHILDREN, MAX_AREAS>,
    tasks: TaskTable<MAX_TASKS>,
    scheduler: Scheduler<MAX_TASKS>,
}

impl<
        const MAX_PROCS: usize,
        const MAX_CHILDREN: usize,
        const MAX_AREAS: usize,
        const MAX_TASKS: usize,
    > ProcessTrapRuntime<MAX_PROCS, MAX_CHILDREN, MAX_AREAS, MAX_TASKS>
{
    pub const fn new() -> Self {
        Self {
            processes: ProcessTable::new(),
            tasks: TaskTable::new(),
            scheduler: Scheduler::new(),
        }
    }

    pub fn create_process(
        &mut self,
        parent: Option<usize>,
        vm: VmSpace<MAX_AREAS>,
    ) -> Result<usize, ProcError> {
        self.processes.create_process(parent, vm)
    }

    pub fn add_runnable_task(
        &mut self,
        tid: usize,
        owner_pid: usize,
        frame: TrapFrame,
    ) -> Result<(), ProcError> {
        self.tasks
            .add_task(TaskControlBlock::new(tid, owner_pid, frame))?;
        self.scheduler
            .add_runnable(tid, frame)
            .map_err(|_| ProcError::TableFull)
    }

    pub fn start_first_task(&mut self) {
        let _ = self.scheduler.on_tick();
    }

    pub fn dispatch_exit(
        &mut self,
        frame: &mut TrapFrame,
        current_tid: usize,
        code: i32,
    ) -> TrapDispatchResult {
        dispatch_exit_outcome_with_process_runtime(
            frame,
            &mut self.processes,
            &mut self.tasks,
            &mut self.scheduler,
            current_tid,
            code,
        )
    }

    pub fn process_state(&self, pid: usize) -> Result<crate::process_model::ProcState, ProcError> {
        Ok(self.processes.get(pid)?.state)
    }

    pub fn process_exit_code(&self, pid: usize) -> Result<Option<i32>, ProcError> {
        Ok(self.processes.get(pid)?.exit_code)
    }

    pub fn task_state(&self, tid: usize) -> Option<TaskState> {
        self.scheduler.task_state(tid)
    }

    pub fn current_tid(&self) -> Option<usize> {
        self.scheduler.current_tid()
    }
}


pub const GLOBAL_TRAP_RUNTIME_TASKS: usize = 4;

struct GlobalTrapRuntime {
    inner: UnsafeCell<TrapRuntime<GLOBAL_TRAP_RUNTIME_TASKS>>,
}

unsafe impl Sync for GlobalTrapRuntime {}

impl GlobalTrapRuntime {
    pub const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(TrapRuntime::new()),
        }
    }

    fn with<R>(&self, f: impl FnOnce(&mut TrapRuntime<GLOBAL_TRAP_RUNTIME_TASKS>) -> R) -> R {
        let _guard = IrqGuard::new();
        let runtime = unsafe { &mut *self.inner.get() };
        f(runtime)
    }
}

static GLOBAL_TRAP_RUNTIME: GlobalTrapRuntime = GlobalTrapRuntime::new();

pub fn with_global_trap_runtime<R>(
    f: impl FnOnce(&mut TrapRuntime<GLOBAL_TRAP_RUNTIME_TASKS>) -> R,
) -> R {
    GLOBAL_TRAP_RUNTIME.with(f)
}

pub fn reset_global_trap_runtime() {
    with_global_trap_runtime(|runtime| {
        *runtime = TrapRuntime::new();
    });
}

pub fn install_initial_user_task(tid: usize, frame: TrapFrame) -> Result<(), SchedError> {
    with_global_trap_runtime(|runtime| {
        *runtime = TrapRuntime::new();
        runtime.add_runnable(tid, frame)?;
        runtime.start_first_task();
        Ok(())
    })
}
