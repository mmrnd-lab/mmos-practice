// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::{
    process_model::{ProcError, ProcessTable, TaskTable},
    scheduler::ScheduleDecision,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressSpaceSwitch {
    pub prev_tid: usize,
    pub next_tid: usize,
    pub next_satp: usize,
}

pub fn address_space_switch_for_decision<
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
>(
    proc_table: &ProcessTable<MAX_PROCS, MAX_CHILDREN, MAX_AREAS>,
    task_table: &TaskTable<MAX_TASKS>,
    decision: ScheduleDecision,
) -> Result<Option<AddressSpaceSwitch>, ProcError> {
    match decision {
        ScheduleDecision::Switched { prev_tid, next_tid } => {
            let next_task = task_table.get(next_tid)?;
            let next_process = proc_table.get(next_task.owner_pid)?;
            Ok(Some(AddressSpaceSwitch {
                prev_tid: prev_tid,
                next_tid: next_tid,
                next_satp: next_process.satp,
            }))
        }
        ScheduleDecision::Idle { .. } => Ok(None),
    }
}
