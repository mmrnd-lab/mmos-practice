// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::scheduler::TaskState;
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
}

#[derive(Debug, Clone, Copy)]
pub struct TaskTable<const MAX_TASKS: usize> {
    slots: [Option<TaskControlBlock>; MAX_TASKS],
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
}
