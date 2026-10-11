// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::scheduler::{ScheduleDecision, Scheduler, TaskState};
use crate::trap::TrapFrame;
use crate::vm::VmSpace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcState {
    Alive,
    Zombie,
    Reaped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcError {
    ChildListFull,
    NoChild,
    NotZombie,
    TableFull,
    MissingAddressSpace,
    MissingProcess,
    MissingTask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessControlBlock<const MAX_CHILDREN: usize, const MAX_AREAS: usize> {
    pub pid: usize,
    pub parent: Option<usize>,
    pub children: [Option<usize>; MAX_CHILDREN],
    pub exit_code: Option<i32>,
    pub state: ProcState,
    pub vm: VmSpace<MAX_AREAS>,
    pub address_space_id: Option<usize>,
    pub satp: usize,
}

impl<const MAX_CHILDREN: usize, const MAX_AREAS: usize> ProcessControlBlock<MAX_CHILDREN, MAX_AREAS> {
    pub fn new(pid: usize, parent: Option<usize>, vm: VmSpace<MAX_AREAS>, satp: usize) -> Self {
        Self {
            pid,
            parent,
            children: [None; MAX_CHILDREN],
            exit_code: None,
            state: ProcState::Alive,
            vm,
            address_space_id: None,
            satp,
        }
    }

    pub fn add_child(&mut self, child_pid: usize) -> Result<(), ProcError> {
        let mut idx = 0usize;
        while idx < MAX_CHILDREN {
            if self.children[idx].is_none() {
                self.children[idx] = Some(child_pid);
                return Ok(());
            }
            idx += 1;
        }
        Err(ProcError::ChildListFull)
    }

    pub fn mark_zombie(&mut self, code: i32) {
        self.exit_code = Some(code);
        self.state = ProcState::Zombie;
    }

    pub fn reap(&mut self) -> Result<i32, ProcError> {
        if self.state != ProcState::Zombie {
            return Err(ProcError::NotZombie);
        }
        let code = self.exit_code.unwrap_or(-1);
        self.state = ProcState::Reaped;
        Ok(code)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TaskControlBlock {
    pub tid: usize,
    pub owner_pid: usize,
    pub trapframe: TrapFrame,
    pub sched_state: TaskState,
    pub runtime_ticks: u64,
}

impl TaskControlBlock {
    pub fn new(tid: usize, owner_pid: usize, trapframe: TrapFrame) -> Self {
        Self {
            tid,
            owner_pid,
            trapframe,
            sched_state: TaskState::Runnable,
            runtime_ticks: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProcessTable<const MAX_PROCS: usize, const MAX_CHILDREN: usize, const MAX_AREAS: usize> {
    slots: [Option<ProcessControlBlock<MAX_CHILDREN, MAX_AREAS>>; MAX_PROCS],
    next_pid: usize,
}

impl<const MAX_PROCS: usize, const MAX_CHILDREN: usize, const MAX_AREAS: usize>
    ProcessTable<MAX_PROCS, MAX_CHILDREN, MAX_AREAS>
{
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_PROCS],
            next_pid: 1,
        }
    }

    pub fn create_process(
        &mut self,
        parent: Option<usize>,
        vm: VmSpace<MAX_AREAS>,
    ) -> Result<usize, ProcError> {
        self.create_process_with_satp(parent, vm, 0)
    }

    pub fn create_process_with_satp(
        &mut self,
        parent: Option<usize>,
        vm: VmSpace<MAX_AREAS>,
        satp: usize,
    ) -> Result<usize, ProcError> {
        let mut idx = 0usize;
        while idx < MAX_PROCS {
            if self.slots[idx].is_none() {
                let pid = self.next_pid;
                self.next_pid = self.next_pid.wrapping_add(1);
                self.slots[idx] = Some(ProcessControlBlock::new(pid, parent, vm, satp));
                return Ok(pid);
            }
            idx += 1;
        }
        Err(ProcError::TableFull)
    }

    pub fn update_satp(&mut self, pid: usize, satp: usize) -> Result<(), ProcError> {
        let process = self.get_mut(pid)?;
        process.satp = satp;
        Ok(())
    }

    pub fn bind_address_space(
        &mut self,
        pid: usize,
        address_space_id: usize,
    ) -> Result<(), ProcError> {
        let process = self.get_mut(pid)?;
        process.address_space_id = Some(address_space_id);
        Ok(())
    }

    pub fn reap_process(&mut self, pid: usize) -> Result<i32, ProcError> {
        let process = self.get_mut(pid)?;
        process.reap()
    }

    pub fn get(
        &self,
        pid: usize,
    ) -> Result<&ProcessControlBlock<MAX_CHILDREN, MAX_AREAS>, ProcError> {
        let mut idx = 0usize;
        while idx < MAX_PROCS {
            if let Some(pcb) = self.slots[idx].as_ref() {
                if pcb.pid == pid {
                    return Ok(pcb);
                }
            }
            idx += 1;
        }
        Err(ProcError::MissingProcess)
    }

    pub fn get_mut(
        &mut self,
        pid: usize,
    ) -> Result<&mut ProcessControlBlock<MAX_CHILDREN, MAX_AREAS>, ProcError> {
        let mut idx = 0usize;
        while idx < MAX_PROCS {
            if let Some(pcb) = self.slots[idx] {
                if pcb.pid == pid {
                    return Ok(self.slots[idx].as_mut().expect("checked above"));
                }
            }
            idx += 1;
        }
        Err(ProcError::MissingProcess)
    }

    pub fn has_child(&self, parent_pid: usize) -> Result<bool, ProcError> {
        let children = self.get(parent_pid)?.children;
        let mut idx = 0usize;
        while idx < MAX_CHILDREN {
            if children[idx].is_some() {
                return Ok(true);
            }
            idx += 1;
        }
        Ok(false)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TaskTable<const MAX_TASKS: usize> {
    slots: [Option<TaskControlBlock>; MAX_TASKS],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessExitRuntime {
    pub owner_pid: usize,
    pub exit_code: i32,
    pub tasks_marked: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessExitScheduleRuntime {
    pub exit: ProcessExitRuntime,
    pub scheduler_tasks_marked: usize,
    pub decision: ScheduleDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessReapRuntime {
    pub pid: usize,
    pub exit_code: i32,
    pub tasks_reaped: usize,
    pub scheduler_tasks_reaped: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessWaitReapRuntime {
    pub parent_pid: usize,
    pub child_pid: usize,
    pub exit_code: i32,
    pub tasks_reaped: usize,
    pub scheduler_tasks_reaped: usize,
}

impl<const MAX_TASKS: usize> TaskTable<MAX_TASKS> {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_TASKS],
        }
    }

    pub fn add_task(&mut self, task: TaskControlBlock) -> Result<(), ProcError> {
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if self.slots[idx].is_none() {
                self.slots[idx] = Some(task);
                return Ok(());
            }
            idx += 1;
        }
        Err(ProcError::TableFull)
    }

    pub fn get(&self, tid: usize) -> Result<&TaskControlBlock, ProcError> {
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if let Some(tcb) = self.slots[idx].as_ref() {
                if tcb.tid == tid {
                    return Ok(tcb);
                }
            }
            idx += 1;
        }
        Err(ProcError::MissingTask)
    }

    pub fn get_mut(&mut self, tid: usize) -> Result<&mut TaskControlBlock, ProcError> {
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if let Some(tcb) = self.slots[idx] {
                if tcb.tid == tid {
                    return Ok(self.slots[idx].as_mut().expect("checked above"));
                }
            }
            idx += 1;
        }
        Err(ProcError::MissingTask)
    }

    pub fn owner_pid(&self, tid: usize) -> Result<usize, ProcError> {
        Ok(self.get(tid)?.owner_pid)
    }

    pub fn mark_owner_tasks_state(&mut self, owner_pid: usize, state: TaskState) -> usize {
        let mut marked = 0usize;
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if let Some(mut tcb) = self.slots[idx] {
                if tcb.owner_pid == owner_pid {
                    tcb.sched_state = state;
                    self.slots[idx] = Some(tcb);
                    marked += 1;
                }
            }
            idx += 1;
        }
        marked
    }

    pub fn count_owner_tasks_with_state(&self, owner_pid: usize, state: TaskState) -> usize {
        let mut count = 0usize;
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if let Some(tcb) = self.slots[idx] {
                if tcb.owner_pid == owner_pid && tcb.sched_state == state {
                    count += 1;
                }
            }
            idx += 1;
        }
        count
    }

    pub fn mark_owner_scheduler_tasks_state_except<const SCHED_TASKS: usize>(
        &self,
        owner_pid: usize,
        except_tid: usize,
        scheduler: &mut Scheduler<SCHED_TASKS>,
        state: TaskState,
    ) -> usize {
        let mut marked = 0usize;
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if let Some(tcb) = self.slots[idx] {
                if tcb.owner_pid == owner_pid && tcb.tid != except_tid {
                    if scheduler.set_state(tcb.tid, state) {
                        marked += 1;
                    }
                }
            }
            idx += 1;
        }
        marked
    }

    pub fn remove_owner_tasks(&mut self, owner_pid: usize) -> usize {
        let mut removed = 0usize;
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if let Some(tcb) = self.slots[idx] {
                if tcb.owner_pid == owner_pid {
                    self.slots[idx] = None;
                    removed += 1;
                }
            }
            idx += 1;
        }
        removed
    }

    pub fn remove_owner_scheduler_tasks<const SCHED_TASKS: usize>(
        &self,
        owner_pid: usize,
        scheduler: &mut Scheduler<SCHED_TASKS>,
    ) -> usize {
        let mut removed = 0usize;
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if let Some(tcb) = self.slots[idx] {
                if tcb.owner_pid == owner_pid && scheduler.remove_task(tcb.tid) {
                    removed += 1;
                }
            }
            idx += 1;
        }
        removed
    }
}

pub fn exit_process_from_task<
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
>(
    processes: &mut ProcessTable<MAX_PROCS, MAX_CHILDREN, MAX_AREAS>,
    tasks: &mut TaskTable<MAX_TASKS>,
    current_tid: usize,
    code: i32,
) -> Result<ProcessExitRuntime, ProcError> {
    let owner_pid = tasks.owner_pid(current_tid)?;
    let process = processes.get_mut(owner_pid)?;
    process.mark_zombie(code);
    let tasks_marked = tasks.mark_owner_tasks_state(owner_pid, TaskState::Zombie);

    Ok(ProcessExitRuntime {
        owner_pid,
        exit_code: code,
        tasks_marked,
    })
}

pub fn exit_process_from_task_with_scheduler<
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
    const SCHED_TASKS: usize,
>(
    processes: &mut ProcessTable<MAX_PROCS, MAX_CHILDREN, MAX_AREAS>,
    tasks: &mut TaskTable<MAX_TASKS>,
    scheduler: &mut Scheduler<SCHED_TASKS>,
    current_tid: usize,
    code: i32,
) -> Result<ProcessExitScheduleRuntime, ProcError> {
    let exit = exit_process_from_task(processes, tasks, current_tid, code)?;
    let sibling_scheduler_tasks = tasks.mark_owner_scheduler_tasks_state_except(
        exit.owner_pid,
        current_tid,
        scheduler,
        TaskState::Zombie,
    );
    let decision = scheduler
        .schedule_after_current_exited()
        .ok_or(ProcError::MissingTask)?;

    Ok(ProcessExitScheduleRuntime {
        exit,
        scheduler_tasks_marked: sibling_scheduler_tasks + 1,
        decision,
    })
}

pub fn reap_exited_process<
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
    const SCHED_TASKS: usize,
>(
    processes: &mut ProcessTable<MAX_PROCS, MAX_CHILDREN, MAX_AREAS>,
    tasks: &mut TaskTable<MAX_TASKS>,
    scheduler: &mut Scheduler<SCHED_TASKS>,
    pid: usize,
) -> Result<ProcessReapRuntime, ProcError> {
    let exit_code = processes.reap_process(pid)?;
    let scheduler_tasks_reaped = tasks.remove_owner_scheduler_tasks(pid, scheduler);
    let tasks_reaped = tasks.remove_owner_tasks(pid);
    Ok(ProcessReapRuntime {
        pid,
        exit_code,
        tasks_reaped,
        scheduler_tasks_reaped,
    })
}

pub fn wait_reap_zombie_child<
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
    const SCHED_TASKS: usize,
>(
    processes: &mut ProcessTable<MAX_PROCS, MAX_CHILDREN, MAX_AREAS>,
    tasks: &mut TaskTable<MAX_TASKS>,
    scheduler: &mut Scheduler<SCHED_TASKS>,
    parent_pid: usize,
) -> Result<ProcessWaitReapRuntime, ProcError> {
    let children = processes.get(parent_pid)?.children;
    let mut child_idx = 0usize;
    while child_idx < MAX_CHILDREN {
        if let Some(child_pid) = children[child_idx] {
            if processes.get(child_pid)?.state == ProcState::Zombie {
                let reap = reap_exited_process(processes, tasks, scheduler, child_pid)?;
                let parent = processes.get_mut(parent_pid)?;
                parent.children[child_idx] = None;
                return Ok(ProcessWaitReapRuntime {
                    parent_pid,
                    child_pid,
                    exit_code: reap.exit_code,
                    tasks_reaped: reap.tasks_reaped,
                    scheduler_tasks_reaped: reap.scheduler_tasks_reaped,
                });
            }
        }
        child_idx += 1;
    }
    Err(ProcError::NotZombie)
}
