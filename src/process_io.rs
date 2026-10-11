// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::char_device::CharacterDevice;
use crate::devfs::DeviceNamespace;
use crate::fd_table::{stdio_table_with_capacity, FileDescriptorTable, FileError, OpenMode};
use crate::process_model::{ProcError, TaskControlBlock};

#[derive(Clone, Copy)]
pub struct ProcessIoContext<const MAX_FDS: usize> {
    fd_table: FileDescriptorTable<MAX_FDS>,
}

impl<const MAX_FDS: usize> ProcessIoContext<MAX_FDS> {
    pub fn with_stdio(console: &'static dyn CharacterDevice) -> Self {
        Self {
            fd_table: stdio_table_with_capacity(console),
        }
    }

    pub fn fd_table(&self) -> &FileDescriptorTable<MAX_FDS> {
        &self.fd_table
    }

    pub fn fd_table_mut(&mut self) -> &mut FileDescriptorTable<MAX_FDS> {
        &mut self.fd_table
    }

    pub fn open_path(
        &mut self,
        ns: &DeviceNamespace<'_>,
        path: &str,
        mode: OpenMode,
    ) -> Result<usize, FileError> {
        self.fd_table.open_path(ns, path, mode)
    }

    pub fn write_fd(&self, fd: usize, bytes: &[u8]) -> Result<usize, FileError> {
        self.fd_table.write(fd, bytes)
    }
}

#[derive(Clone, Copy)]
pub struct CurrentProcessIoContext<const MAX_FDS: usize> {
    pid: usize,
    io: ProcessIoContext<MAX_FDS>,
}

impl<const MAX_FDS: usize> CurrentProcessIoContext<MAX_FDS> {
    pub fn with_stdio(pid: usize, console: &'static dyn CharacterDevice) -> Self {
        Self {
            pid,
            io: ProcessIoContext::with_stdio(console),
        }
    }

    pub const fn pid(&self) -> usize {
        self.pid
    }

    pub fn io(&self) -> &ProcessIoContext<MAX_FDS> {
        &self.io
    }

    pub fn io_mut(&mut self) -> &mut ProcessIoContext<MAX_FDS> {
        &mut self.io
    }

    pub fn open_path(
        &mut self,
        ns: &DeviceNamespace<'_>,
        path: &str,
        mode: OpenMode,
    ) -> Result<usize, FileError> {
        self.io.open_path(ns, path, mode)
    }

    pub fn write_fd(&self, fd: usize, bytes: &[u8]) -> Result<usize, FileError> {
        self.io.write_fd(fd, bytes)
    }
}


pub struct ProcessIoTable<const MAX_PROCS: usize, const MAX_FDS: usize> {
    slots: [Option<CurrentProcessIoContext<MAX_FDS>>; MAX_PROCS],
}

impl<const MAX_PROCS: usize, const MAX_FDS: usize> ProcessIoTable<MAX_PROCS, MAX_FDS> {
    pub const fn new() -> Self {
        Self { slots: [None; MAX_PROCS] }
    }

    pub fn insert(&mut self, io: CurrentProcessIoContext<MAX_FDS>) -> Result<(), ProcError> {
        let mut idx = 0usize;
        while idx < MAX_PROCS {
            if self.slots[idx].is_none() {
                self.slots[idx] = Some(io);
                return Ok(());
            }
            idx += 1;
        }
        Err(ProcError::TableFull)
    }

    pub fn insert_stdio(
        &mut self,
        pid: usize,
        console: &'static dyn CharacterDevice,
    ) -> Result<(), ProcError> {
        self.insert(CurrentProcessIoContext::with_stdio(pid, console))
    }

    pub fn get(&self, pid: usize) -> Result<&CurrentProcessIoContext<MAX_FDS>, ProcError> {
        let mut idx = 0usize;
        while idx < MAX_PROCS {
            if let Some(io) = self.slots[idx].as_ref() {
                if io.pid() == pid {
                    return Ok(io);
                }
            }
            idx += 1;
        }
        Err(ProcError::MissingProcess)
    }

    pub fn get_mut(
        &mut self,
        pid: usize,
    ) -> Result<&mut CurrentProcessIoContext<MAX_FDS>, ProcError> {
        let mut idx = 0usize;
        while idx < MAX_PROCS {
            if let Some(io) = self.slots[idx].as_ref() {
                if io.pid() == pid {
                    return Ok(self.slots[idx].as_mut().expect("checked above"));
                }
            }
            idx += 1;
        }
        Err(ProcError::MissingProcess)
    }

    pub fn current_for_task(
        &self,
        task: &TaskControlBlock,
    ) -> Result<&CurrentProcessIoContext<MAX_FDS>, ProcError> {
        self.get(task.owner_pid)
    }

    pub fn current_for_task_mut(
        &mut self,
        task: &TaskControlBlock,
    ) -> Result<&mut CurrentProcessIoContext<MAX_FDS>, ProcError> {
        self.get_mut(task.owner_pid)
    }
}
