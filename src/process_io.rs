// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::char_device::CharacterDevice;
use crate::devfs::DeviceNamespace;
use crate::fd_table::{stdio_table_with_capacity, FileDescriptorTable, FileError, OpenMode};

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
