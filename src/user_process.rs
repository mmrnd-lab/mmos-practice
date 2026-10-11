// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::{
    process_model::{ProcError, ProcessTable, TaskControlBlock, TaskTable},
    scheduler::{SchedError, Scheduler},
    user_launch::UserLaunchPlan,
    vm::VmSpace,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserProcessError {
    Process(ProcError),
    Scheduler(SchedError),
}

impl From<ProcError> for UserProcessError {
    fn from(value: ProcError) -> Self {
        Self::Process(value)
    }
}

impl From<SchedError> for UserProcessError {
    fn from(value: SchedError) -> Self {
        Self::Scheduler(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatedUserProcess {
    pub pid: usize,
    pub tid: usize,
    pub entry: usize,
    pub stack_top: usize,
    pub satp: usize,
}

pub fn create_user_process_from_launch<
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
    const MAX_SCHED_TASKS: usize,
>(
    proc_table: &mut ProcessTable<MAX_PROCS, MAX_CHILDREN, MAX_AREAS>,
    task_table: &mut TaskTable<MAX_TASKS>,
    scheduler: &mut Scheduler<MAX_SCHED_TASKS>,
    parent: Option<usize>,
    vm: VmSpace<MAX_AREAS>,
    launch: UserLaunchPlan,
) -> Result<CreatedUserProcess, UserProcessError> {
    let pid = proc_table.create_process_with_satp(parent, vm, launch.satp)?;
    if let Some(parent_pid) = parent {
        proc_table.get_mut(parent_pid)?.add_child(pid)?;
    }

    let tid = pid;
    task_table.add_task(TaskControlBlock::new(tid, pid, launch.trapframe))?;
    scheduler.add_runnable(tid, launch.trapframe)?;

    Ok(CreatedUserProcess {
        pid,
        tid,
        entry: launch.trapframe.sepc,
        stack_top: launch.sp,
        satp: launch.satp,
    })
}
