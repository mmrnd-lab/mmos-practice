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


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedAddressSpaceSwitch {
    pub prev_tid: usize,
    pub next_tid: usize,
    pub active_satp: usize,
}

pub fn apply_address_space_switch(
    active_satp: &mut usize,
    switch: AddressSpaceSwitch,
) -> AppliedAddressSpaceSwitch {
    *active_satp = switch.next_satp;
    AppliedAddressSpaceSwitch {
        prev_tid: switch.prev_tid,
        next_tid: switch.next_tid,
        active_satp: switch.next_satp,
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SatpCommitPlan {
    pub prev_satp: usize,
    pub next_satp: usize,
    pub needs_sfence_vma: bool,
}

pub fn prepare_satp_commit(prev_satp: usize, applied: AppliedAddressSpaceSwitch) -> SatpCommitPlan {
    SatpCommitPlan {
        prev_satp,
        next_satp: applied.active_satp,
        needs_sfence_vma: prev_satp != applied.active_satp,
    }
}

#[cfg(target_arch = "riscv64")]
pub unsafe fn write_satp_and_flush(next_satp: usize) {
    core::arch::asm!(
        "csrw satp, {satp}",
        "sfence.vma x0, x0",
        satp = in(reg) next_satp,
        options(nostack, preserves_flags)
    );
}

#[cfg(not(target_arch = "riscv64"))]
pub unsafe fn write_satp_and_flush(_next_satp: usize) {}
