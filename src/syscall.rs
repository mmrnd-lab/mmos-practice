// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

pub const SYS_WRITE: usize = 64;
pub const SYS_EXIT: usize = 93;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syscall {
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
        Some(Syscall::Write {
            fd,
            ptr: _,
            len,
        }) => {
            if fd == 1 {
                SyscallResult::Continue(len as isize)
            } else {
                SyscallResult::Continue(-1)
            }
        }
        Some(Syscall::Exit { code }) => SyscallResult::Exit(code),
        None => SyscallResult::Unsupported,
    }
}
