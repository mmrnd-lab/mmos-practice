// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::process_model::{ProcError, ProcessTable, TaskControlBlock, TaskTable};
use crate::trap::TrapFrame;
use crate::vm::VmSpace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForkExecWaitDemo {
    pub parent_pid: usize,
    pub child_pid: usize,
    pub child_tid: usize,
    pub exec_entry: usize,
    pub exec_stack_top: usize,
    pub reap_code: i32,
}

pub fn run_fork_exec_wait_demo() -> Result<ForkExecWaitDemo, ProcError> {
    // Keep runtime stack usage small on bare-metal demo path.
    let mut proc_table = ProcessTable::<2, 2, 2>::new();
    let mut task_table = TaskTable::<2>::new();

    let parent_pid = proc_table.create_process(None, VmSpace::<2>::new())?;
    let child_pid = proc_table.create_process(Some(parent_pid), VmSpace::<2>::new())?;

    {
        let parent = proc_table.get_mut(parent_pid)?;
        parent.add_child(child_pid)?;
    }

    let child_tid = 1usize;
    task_table.add_task(TaskControlBlock::new(
        child_tid,
        child_pid,
        TrapFrame::default(),
        0,
    ))?;

    let exec_entry = 0x8040_1000usize;
    let exec_stack_top = 0x8050_1000usize;
    {
        // Simulate exec() rewriting the runnable context for child.
        let child_task = task_table.get_mut(child_tid)?;
        child_task.trapframe.sepc = exec_entry;
        child_task.trapframe.set_sp(exec_stack_top);
        child_task.runtime_ticks = child_task.runtime_ticks.wrapping_add(1);
    }

    {
        let child = proc_table.get_mut(child_pid)?;
        child.mark_zombie(0);
    }
    let reap_code = {
        let child = proc_table.get_mut(child_pid)?;
        child.reap()?
    };

    Ok(ForkExecWaitDemo {
        parent_pid,
        child_pid,
        child_tid,
        exec_entry,
        exec_stack_top,
        reap_code,
    })
}
