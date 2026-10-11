// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

pub const SYS_OPENAT: usize = 56;
pub const SYS_WRITE: usize = 64;
pub const SYS_EXIT: usize = 93;

const O_ACCMODE: usize = 0o3;
const O_RDONLY: usize = 0;
const O_WRONLY: usize = 1;
const O_RDWR: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syscall {
    OpenAt { dirfd: usize, path_ptr: usize, flags: usize, mode: usize },
    Write { fd: usize, ptr: usize, len: usize },
    Exit { code: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallResult {
    Continue(isize),
    Exit(i32),
    Unsupported,
}

pub fn decode_syscall(id: usize, a0: usize, a1: usize, a2: usize) -> Option<Syscall> {
    match id {
        SYS_OPENAT => Some(Syscall::OpenAt {
            dirfd: a0,
            path_ptr: a1,
            flags: a2,
            mode: 0,
        }),
        SYS_WRITE => Some(Syscall::Write {
            fd: a0,
            ptr: a1,
            len: a2,
        }),
        SYS_EXIT => Some(Syscall::Exit { code: a0 as i32 }),
        _ => None,
    }
}

pub fn handle_syscall(id: usize, a0: usize, a1: usize, a2: usize) -> SyscallResult {
    match decode_syscall(id, a0, a1, a2) {
        Some(Syscall::OpenAt { .. }) => SyscallResult::Unsupported,
        Some(Syscall::Write { fd, ptr, len }) => handle_write(fd, ptr, len),
        Some(Syscall::Exit { code }) => SyscallResult::Exit(code),
        None => SyscallResult::Unsupported,
    }
}

pub fn open_mode_from_flags(flags: usize) -> Option<crate::fd_table::OpenMode> {
    match flags & O_ACCMODE {
        O_RDONLY => Some(crate::fd_table::OpenMode::ReadOnly),
        O_WRONLY => Some(crate::fd_table::OpenMode::WriteOnly),
        O_RDWR => Some(crate::fd_table::OpenMode::ReadWrite),
        _ => None,
    }
}

pub fn handle_open_path<const N: usize>(
    table: &mut crate::fd_table::FileDescriptorTable<N>,
    ns: &crate::devfs::DeviceNamespace<'_>,
    path: &str,
    flags: usize,
) -> SyscallResult {
    match open_mode_from_flags(flags).and_then(|mode| table.open_path(ns, path, mode).ok()) {
        Some(fd) => SyscallResult::Continue(fd as isize),
        None => SyscallResult::Continue(-1),
    }
}

#[cfg(target_arch = "riscv64")]
fn handle_write(fd: usize, ptr: usize, len: usize) -> SyscallResult {
    if len == 0 {
        return SyscallResult::Continue(0);
    }

    let capped = core::cmp::min(len, 1024);
    let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, capped) };
    let table = crate::fd_table::stdio_table(crate::console::device());
    match table.write(fd, bytes) {
        Ok(written) => SyscallResult::Continue(written as isize),
        Err(_) => SyscallResult::Continue(-1),
    }
}

#[cfg(not(target_arch = "riscv64"))]
fn handle_write(fd: usize, _ptr: usize, len: usize) -> SyscallResult {
    if fd == 1 {
        SyscallResult::Continue(len as isize)
    } else {
        SyscallResult::Continue(-1)
    }
}
