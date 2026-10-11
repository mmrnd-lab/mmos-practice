// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::{
    process_model::{ProcError, ProcessTable, TaskTable},
    scheduler::ScheduleDecision,
};
use crate::sv39::SATP_MODE_SV39;
use crate::trap::{KernelTrapStackError, TrapEntryScratch, TrapFrame};
use crate::user_vm::{AddressSpaceTableError, ProcessAddressSpaceTable};

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

pub fn address_space_switch_for_decision_with_spaces<
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
    const MAX_SPACES: usize,
>(
    proc_table: &ProcessTable<MAX_PROCS, MAX_CHILDREN, MAX_AREAS>,
    task_table: &TaskTable<MAX_TASKS>,
    spaces: &ProcessAddressSpaceTable<MAX_SPACES>,
    decision: ScheduleDecision,
) -> Result<Option<AddressSpaceSwitch>, ProcError> {
    match decision {
        ScheduleDecision::Switched { prev_tid, next_tid } => {
            let next_task = task_table.get(next_tid)?;
            let next_process = proc_table.get(next_task.owner_pid)?;
            let next_satp = match next_process.address_space_id {
                Some(id) => spaces.satp(id).map_err(address_space_error_to_proc_error)?,
                None => next_process.satp,
            };
            Ok(Some(AddressSpaceSwitch {
                prev_tid: prev_tid,
                next_tid: next_tid,
                next_satp,
            }))
        }
        ScheduleDecision::Idle { .. } => Ok(None),
    }
}

fn address_space_error_to_proc_error(error: AddressSpaceTableError) -> ProcError {
    match error {
        AddressSpaceTableError::TableFull => ProcError::TableFull,
        AddressSpaceTableError::MissingAddressSpace => ProcError::MissingAddressSpace,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SatpHardwareCommitPlan {
    pub next_satp: usize,
    pub flush_after_write: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SatpHardwareCommitError {
    Unchanged,
    UnsupportedMode { mode: usize },
}

pub fn prepare_satp_hardware_commit(
    plan: SatpCommitPlan,
) -> Result<SatpHardwareCommitPlan, SatpHardwareCommitError> {
    if !plan.needs_sfence_vma {
        return Err(SatpHardwareCommitError::Unchanged);
    }

    let mode = plan.next_satp >> 60;
    if mode != SATP_MODE_SV39 {
        return Err(SatpHardwareCommitError::UnsupportedMode { mode });
    }

    Ok(SatpHardwareCommitPlan {
        next_satp: plan.next_satp,
        flush_after_write: true,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SatpHardwareCommitResult {
    pub active_satp: usize,
    pub flushed_tlb: bool,
}

pub unsafe fn commit_satp_hardware(plan: SatpHardwareCommitPlan) -> SatpHardwareCommitResult {
    unsafe {
        write_satp_and_flush(plan.next_satp);
    }
    SatpHardwareCommitResult {
        active_satp: plan.next_satp,
        flushed_tlb: plan.flush_after_write,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SatpHardwareCommitProbe {
    pub committed_satp: usize,
    pub restored_satp: usize,
    pub flushed_on_commit: bool,
    pub flushed_on_restore: bool,
}

pub unsafe fn probe_satp_hardware_commit_roundtrip(
    plan: SatpHardwareCommitPlan,
    restore_satp: usize,
) -> SatpHardwareCommitProbe {
    let committed = unsafe { commit_satp_hardware(plan) };
    unsafe {
        write_satp_and_flush(restore_satp);
    }
    SatpHardwareCommitProbe {
        committed_satp: committed.active_satp,
        restored_satp: restore_satp,
        flushed_on_commit: committed.flushed_tlb,
        flushed_on_restore: true,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserLaunchCsrSequence {
    pub sscratch: usize,
    pub satp: usize,
    pub sfence_vma_after_satp: bool,
    pub sepc: usize,
    pub user_sp: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootedUserLaunchCsrSequence {
    pub root_satp: usize,
    pub sequence: UserLaunchCsrSequence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootedSatpCommitSequence {
    pub switch: AddressSpaceSwitch,
    pub applied: AppliedAddressSpaceSwitch,
    pub commit: SatpCommitPlan,
    pub rooted: RootedUserLaunchCsrSequence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserLaunchCsrError {
    Scratch(KernelTrapStackError),
    UserSpMismatch { frame_sp: usize, scratch_user_sp: usize },
    Satp(SatpHardwareCommitError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootedUserLaunchCsrError {
    Sequence(UserLaunchCsrError),
    SatpMismatch { root_satp: usize, sequence_satp: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootedSatpCommitError {
    SwitchSatpMismatch { switch_satp: usize, root_satp: usize },
    Rooted(RootedUserLaunchCsrError),
}

pub fn prepare_user_launch_csr_sequence(
    frame: &TrapFrame,
    scratch: TrapEntryScratch,
    commit: SatpCommitPlan,
) -> Result<UserLaunchCsrSequence, UserLaunchCsrError> {
    if scratch.user_sp != frame.sp() {
        return Err(UserLaunchCsrError::UserSpMismatch {
            frame_sp: frame.sp(),
            scratch_user_sp: scratch.user_sp,
        });
    }

    let hardware = prepare_satp_hardware_commit(commit).map_err(UserLaunchCsrError::Satp)?;

    Ok(UserLaunchCsrSequence {
        sscratch: scratch.sscratch,
        satp: hardware.next_satp,
        sfence_vma_after_satp: hardware.flush_after_write,
        sepc: frame.sepc,
        user_sp: frame.sp(),
    })
}

pub fn prepare_rooted_user_launch_csr_sequence(
    frame: &TrapFrame,
    scratch: TrapEntryScratch,
    commit: SatpCommitPlan,
    root_satp: usize,
) -> Result<RootedUserLaunchCsrSequence, RootedUserLaunchCsrError> {
    let sequence = prepare_user_launch_csr_sequence(frame, scratch, commit)
        .map_err(RootedUserLaunchCsrError::Sequence)?;

    if sequence.satp != root_satp {
        return Err(RootedUserLaunchCsrError::SatpMismatch {
            root_satp,
            sequence_satp: sequence.satp,
        });
    }

    Ok(RootedUserLaunchCsrSequence {
        root_satp,
        sequence,
    })
}

pub fn prepare_rooted_satp_commit_sequence(
    active_satp: &mut usize,
    switch: AddressSpaceSwitch,
    frame: &TrapFrame,
    scratch: TrapEntryScratch,
    root_satp: usize,
) -> Result<RootedSatpCommitSequence, RootedSatpCommitError> {
    if switch.next_satp != root_satp {
        return Err(RootedSatpCommitError::SwitchSatpMismatch {
            switch_satp: switch.next_satp,
            root_satp,
        });
    }

    let prev_satp = *active_satp;
    let applied = apply_address_space_switch(active_satp, switch);
    let commit = prepare_satp_commit(prev_satp, applied);
    let rooted = prepare_rooted_user_launch_csr_sequence(frame, scratch, commit, root_satp)
        .map_err(RootedSatpCommitError::Rooted)?;

    Ok(RootedSatpCommitSequence {
        switch,
        applied,
        commit,
        rooted,
    })
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
