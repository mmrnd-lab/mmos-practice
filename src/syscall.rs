// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

pub const SYS_OPENAT: usize = 56;
pub const SYS_WRITE: usize = 64;
pub const SYS_EXIT: usize = 93;

pub const EBADF: isize = 9;
pub const EACCES: isize = 13;
pub const EFAULT: isize = 14;
pub const EINVAL: isize = 22;
pub const EIO: isize = 5;

pub fn errno_result(errno: isize) -> SyscallResult {
    SyscallResult::Continue(-errno)
}

pub fn file_error_errno(error: crate::fd_table::FileError) -> isize {
    match error {
        crate::fd_table::FileError::BadFd => EBADF,
        crate::fd_table::FileError::NotReadable | crate::fd_table::FileError::NotWritable => EACCES,
        crate::fd_table::FileError::NotCharacterDevice
        | crate::fd_table::FileError::Lookup(_)
        | crate::fd_table::FileError::Char(_) => EIO,
    }
}

pub fn user_copy_errno(error: crate::user_copy::UserCopyError) -> isize {
    match error {
        crate::user_copy::UserCopyError::BadAddress
        | crate::user_copy::UserCopyError::MissingNul
        | crate::user_copy::UserCopyError::Permission => EFAULT,
        crate::user_copy::UserCopyError::BufferTooSmall => EINVAL,
    }
}

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

pub fn handle_process_open_path<const N: usize>(
    io: &mut crate::process_io::ProcessIoContext<N>,
    ns: &crate::devfs::DeviceNamespace<'_>,
    path: &str,
    flags: usize,
) -> SyscallResult {
    match open_mode_from_flags(flags).and_then(|mode| io.open_path(ns, path, mode).ok()) {
        Some(fd) => SyscallResult::Continue(fd as isize),
        None => SyscallResult::Continue(-1),
    }
}

pub fn handle_process_write<const N: usize>(
    io: &crate::process_io::ProcessIoContext<N>,
    fd: usize,
    bytes: &[u8],
) -> SyscallResult {
    match io.write_fd(fd, bytes) {
        Ok(written) => SyscallResult::Continue(written as isize),
        Err(_) => SyscallResult::Continue(-1),
    }
}

pub fn handle_process_open_user_path<const N: usize>(
    io: &mut crate::process_io::ProcessIoContext<N>,
    ns: &crate::devfs::DeviceNamespace<'_>,
    user: &crate::user_copy::UserSlice<'_>,
    path_ptr: usize,
    flags: usize,
    scratch: &mut [u8],
) -> SyscallResult {
    match user.copyin_cstr(path_ptr, scratch) {
        Ok(path) => handle_process_open_path(io, ns, path, flags),
        Err(_) => SyscallResult::Continue(-1),
    }
}

pub fn handle_process_write_user_buf<const N: usize>(
    io: &crate::process_io::ProcessIoContext<N>,
    user: &crate::user_copy::UserSlice<'_>,
    fd: usize,
    ptr: usize,
    len: usize,
    scratch: &mut [u8],
) -> SyscallResult {
    match user.copyin_bytes(ptr, len, scratch) {
        Ok(n) => handle_process_write(io, fd, &scratch[..n]),
        Err(_) => SyscallResult::Continue(-1),
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


pub fn handle_process_open_checked_user_path<const N: usize, const A: usize>(
    io: &mut crate::process_io::ProcessIoContext<N>,
    ns: &crate::devfs::DeviceNamespace<'_>,
    user: &crate::user_copy::CheckedUserSlice<'_, A>,
    path_ptr: usize,
    flags: usize,
    scratch: &mut [u8],
) -> SyscallResult {
    match user.copyin_cstr(path_ptr, scratch) {
        Ok(path) => handle_process_open_path(io, ns, path, flags),
        Err(_) => SyscallResult::Continue(-1),
    }
}

pub fn handle_process_write_checked_user_buf<const N: usize, const A: usize>(
    io: &crate::process_io::ProcessIoContext<N>,
    user: &crate::user_copy::CheckedUserSlice<'_, A>,
    fd: usize,
    ptr: usize,
    len: usize,
    scratch: &mut [u8],
) -> SyscallResult {
    match user.copyin_bytes(ptr, len, scratch) {
        Ok(n) => handle_process_write(io, fd, &scratch[..n]),
        Err(_) => SyscallResult::Continue(-1),
    }
}

pub fn handle_process_write_checked_user_syscall<const N: usize, const A: usize>(
    io: &crate::process_io::ProcessIoContext<N>,
    user: &crate::user_copy::CheckedUserSlice<'_, A>,
    fd: usize,
    ptr: usize,
    len: usize,
    scratch: &mut [u8],
) -> SyscallResult {
    let copied = match user.copyin_bytes(ptr, len, scratch) {
        Ok(n) => n,
        Err(error) => return errno_result(user_copy_errno(error)),
    };

    match io.write_fd(fd, &scratch[..copied]) {
        Ok(written) => SyscallResult::Continue(written as isize),
        Err(error) => errno_result(file_error_errno(error)),
    }
}
