// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

pub const SYS_OPENAT: usize = 56;
pub const SYS_CLOSE: usize = 57;
pub const SYS_READ: usize = 63;
pub const SYS_WRITE: usize = 64;
pub const SYS_EXIT: usize = 93;
pub const SYS_CLONE: usize = 220;
pub const SYS_EXECVE: usize = 221;
pub const SYS_WAIT4: usize = 260;

pub const EBADF: isize = 9;
pub const ECHILD: isize = 10;
pub const EAGAIN: isize = 11;
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
        crate::fd_table::FileError::Char(crate::char_device::CharError::WouldBlock) => EAGAIN,
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

pub fn process_wait_errno(error: crate::process_model::ProcError) -> isize {
    match error {
        crate::process_model::ProcError::NoChild
        | crate::process_model::ProcError::NotZombie
        | crate::process_model::ProcError::MissingProcess => ECHILD,
        crate::process_model::ProcError::MissingTask => EINVAL,
        crate::process_model::ProcError::ChildListFull
        | crate::process_model::ProcError::TableFull
        | crate::process_model::ProcError::MissingAddressSpace => EIO,
    }
}

const O_ACCMODE: usize = 0o3;
const O_RDONLY: usize = 0;
const O_WRONLY: usize = 1;
const O_RDWR: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syscall {
    OpenAt { dirfd: usize, path_ptr: usize, flags: usize, mode: usize },
    Close { fd: usize },
    Read { fd: usize, ptr: usize, len: usize },
    Write { fd: usize, ptr: usize, len: usize },
    Exit { code: i32 },
    Clone { flags: usize, stack_ptr: usize },
    Execv { path_ptr: usize, argv_ptr: usize, envp_ptr: usize },
    Wait4 { pid: isize, status_ptr: usize, options: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallResult {
    Continue(isize),
    Exit(i32),
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockingReadResult {
    Ready(SyscallResult),
    Blocked,
}

pub fn decode_syscall(id: usize, a0: usize, a1: usize, a2: usize) -> Option<Syscall> {
    match id {
        SYS_OPENAT => Some(Syscall::OpenAt {
            dirfd: a0,
            path_ptr: a1,
            flags: a2,
            mode: 0,
        }),
        SYS_CLOSE => Some(Syscall::Close { fd: a0 }),
        SYS_READ => Some(Syscall::Read {
            fd: a0,
            ptr: a1,
            len: a2,
        }),
        SYS_WRITE => Some(Syscall::Write {
            fd: a0,
            ptr: a1,
            len: a2,
        }),
        SYS_EXIT => Some(Syscall::Exit { code: a0 as i32 }),
        SYS_CLONE => Some(Syscall::Clone {
            flags: a0,
            stack_ptr: a1,
        }),
        SYS_EXECVE => Some(Syscall::Execv {
            path_ptr: a0,
            argv_ptr: a1,
            envp_ptr: a2,
        }),
        SYS_WAIT4 => Some(Syscall::Wait4 {
            pid: a0 as isize,
            status_ptr: a1,
            options: a2,
        }),
        _ => None,
    }
}

pub fn handle_syscall(id: usize, a0: usize, a1: usize, a2: usize) -> SyscallResult {
    match decode_syscall(id, a0, a1, a2) {
        Some(Syscall::OpenAt { .. }) => SyscallResult::Unsupported,
        Some(Syscall::Close { .. }) => SyscallResult::Unsupported,
        Some(Syscall::Read { .. }) => SyscallResult::Unsupported,
        Some(Syscall::Write { fd, ptr, len }) => handle_write(fd, ptr, len),
        Some(Syscall::Exit { code }) => SyscallResult::Exit(code),
        Some(Syscall::Clone { .. }) => SyscallResult::Unsupported,
        Some(Syscall::Execv { .. }) => SyscallResult::Unsupported,
        Some(Syscall::Wait4 { .. }) => SyscallResult::Unsupported,
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

pub fn handle_process_read<const N: usize>(
    io: &crate::process_io::ProcessIoContext<N>,
    fd: usize,
    out: &mut [u8],
) -> SyscallResult {
    match io.read_fd(fd, out) {
        Ok(read) => SyscallResult::Continue(read as isize),
        Err(error) => errno_result(file_error_errno(error)),
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

pub fn handle_process_close<const N: usize>(
    io: &mut crate::process_io::ProcessIoContext<N>,
    fd: usize,
) -> SyscallResult {
    match io.close_fd(fd) {
        Ok(()) => SyscallResult::Continue(0),
        Err(error) => errno_result(file_error_errno(error)),
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
    let result = crate::user_access::with_user_memory_access(|| {
        let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, capped) };
        let table = crate::fd_table::stdio_table(crate::console::device());
        table.write(fd, bytes)
    });
    match result {
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

pub fn handle_current_process_write_checked_user_syscall<const N: usize, const A: usize>(
    current: &crate::process_io::CurrentProcessIoContext<N>,
    user: &crate::user_copy::CheckedUserSlice<'_, A>,
    fd: usize,
    ptr: usize,
    len: usize,
    scratch: &mut [u8],
) -> SyscallResult {
    handle_process_write_checked_user_syscall(current.io(), user, fd, ptr, len, scratch)
}


pub fn handle_current_process_close<const N: usize>(
    current: &mut crate::process_io::CurrentProcessIoContext<N>,
    fd: usize,
) -> SyscallResult {
    handle_process_close(current.io_mut(), fd)
}


pub fn handle_current_process_read_checked_user_syscall<const N: usize, const A: usize>(
    current: &crate::process_io::CurrentProcessIoContext<N>,
    user: &mut crate::user_copy::CheckedUserSliceMut<'_, A>,
    fd: usize,
    ptr: usize,
    len: usize,
    scratch: &mut [u8],
) -> SyscallResult {
    if len > scratch.len() {
        return errno_result(EINVAL);
    }
    if len == 0 {
        return SyscallResult::Continue(0);
    }

    let read = match current.read_fd(fd, &mut scratch[..len]) {
        Ok(read) => read,
        Err(error) => return errno_result(file_error_errno(error)),
    };

    match user.copyout_bytes(ptr, &scratch[..read]) {
        Ok(n) => SyscallResult::Continue(n as isize),
        Err(error) => errno_result(user_copy_errno(error)),
    }
}


pub fn handle_current_process_blocking_read_checked_user_syscall<
    const N: usize,
    const A: usize,
    const MAX_TASKS: usize,
    const MAX_WAITERS: usize,
>(
    current: &crate::process_io::CurrentProcessIoContext<N>,
    current_tid: usize,
    user: &mut crate::user_copy::CheckedUserSliceMut<'_, A>,
    fd: usize,
    ptr: usize,
    len: usize,
    scratch: &mut [u8],
    scheduler: &mut crate::scheduler::Scheduler<MAX_TASKS>,
    waiters: &mut crate::wait_queue::WaitQueue<MAX_WAITERS>,
) -> BlockingReadResult {
    if len > scratch.len() {
        return BlockingReadResult::Ready(errno_result(EINVAL));
    }
    if len == 0 {
        return BlockingReadResult::Ready(SyscallResult::Continue(0));
    }

    let read = match current.read_fd(fd, &mut scratch[..len]) {
        Ok(read) => read,
        Err(crate::fd_table::FileError::Char(crate::char_device::CharError::WouldBlock)) => {
            if waiters.enqueue(current_tid).is_ok() {
                scheduler.set_state(current_tid, crate::scheduler::TaskState::Blocked);
                return BlockingReadResult::Blocked;
            }
            return BlockingReadResult::Ready(errno_result(EAGAIN));
        }
        Err(error) => return BlockingReadResult::Ready(errno_result(file_error_errno(error))),
    };

    match user.copyout_bytes(ptr, &scratch[..read]) {
        Ok(n) => BlockingReadResult::Ready(SyscallResult::Continue(n as isize)),
        Err(error) => BlockingReadResult::Ready(errno_result(user_copy_errno(error))),
    }
}

pub fn handle_process_wait4_checked_user_syscall<const A: usize>(
    user: &mut crate::user_copy::CheckedUserSliceMut<'_, A>,
    status_ptr: usize,
    report: crate::process_model::ProcessWaitReapRuntime,
) -> SyscallResult {
    if status_ptr != 0 {
        let status = report.exit_code.to_ne_bytes();
        if let Err(error) = user.copyout_bytes(status_ptr, &status) {
            return errno_result(user_copy_errno(error));
        }
    }

    SyscallResult::Continue(report.child_pid as isize)
}

pub fn handle_process_fork_syscall(
    report: crate::trap_runtime::ProcessForkRuntime,
) -> SyscallResult {
    SyscallResult::Continue(report.child_pid as isize)
}

pub fn handle_current_process_clone_syscall<
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
>(
    runtime: &mut crate::trap_runtime::ProcessTrapRuntime<
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >,
    syscall: Syscall,
    parent_frame: &crate::trap::TrapFrame,
    child_tid: usize,
    child_vm: crate::vm::VmSpace<MAX_AREAS>,
) -> SyscallResult {
    let Syscall::Clone { stack_ptr, .. } = syscall else {
        return SyscallResult::Unsupported;
    };

    let mut child_frame = *parent_frame;
    if stack_ptr != 0 {
        child_frame.set_sp(stack_ptr);
    }

    match runtime.fork_current_process(child_tid, child_frame, child_vm) {
        Ok(report) => handle_process_fork_syscall(report),
        Err(error) => errno_result(process_wait_errno(error)),
    }
}

pub fn execv_arg_errno(error: crate::exec_args::ExecvArgError) -> isize {
    match error {
        crate::exec_args::ExecvArgError::Copy(error) => user_copy_errno(error),
        crate::exec_args::ExecvArgError::Stack(_)
        | crate::exec_args::ExecvArgError::TooManyArgs
        | crate::exec_args::ExecvArgError::MissingArgvNull => EINVAL,
    }
}

pub fn exec_path_errno(error: crate::exec_path::ExecPathError) -> isize {
    match error {
        crate::exec_path::ExecPathError::Args(error) => execv_arg_errno(error),
        crate::exec_path::ExecPathError::Load(_) => EIO,
    }
}

pub fn execv_apply_errno(error: crate::exec_flow::ExecvApplyError) -> isize {
    match error {
        crate::exec_flow::ExecvApplyError::Args(error) => execv_arg_errno(error),
        crate::exec_flow::ExecvApplyError::Path(error) => exec_path_errno(error),
        crate::exec_flow::ExecvApplyError::Runtime(error) => process_wait_errno(error),
    }
}

pub fn handle_process_execv_checked_user_syscall<
    const USER_AREAS: usize,
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
>(
    runtime: &mut crate::trap_runtime::ProcessTrapRuntime<
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >,
    tid: usize,
    user: &crate::user_copy::CheckedUserSlice<'_, USER_AREAS>,
    path_ptr: usize,
    argv_ptr: usize,
    vfs: &crate::vfs::Vfs<'_>,
    scratch: &mut [u8],
    stack_bytes: &mut [u8],
    stack_top_va: usize,
    new_vm: crate::vm::VmSpace<MAX_AREAS>,
) -> SyscallResult {
    match crate::exec_flow::apply_execv_from_user_args::<
        USER_AREAS,
        MAX_ARGS,
        ARG_BYTES,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >(
        runtime,
        tid,
        user,
        path_ptr,
        argv_ptr,
        vfs,
        scratch,
        stack_bytes,
        stack_top_va,
        new_vm,
    ) {
        Ok((exec, _report)) => handle_process_execv_syscall(exec),
        Err(error) => errno_result(execv_apply_errno(error)),
    }
}

pub fn handle_current_process_execv_checked_user_syscall<
    const USER_AREAS: usize,
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
>(
    runtime: &mut crate::trap_runtime::ProcessTrapRuntime<
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >,
    user: &crate::user_copy::CheckedUserSlice<'_, USER_AREAS>,
    path_ptr: usize,
    argv_ptr: usize,
    vfs: &crate::vfs::Vfs<'_>,
    scratch: &mut [u8],
    stack_bytes: &mut [u8],
    stack_top_va: usize,
    new_vm: crate::vm::VmSpace<MAX_AREAS>,
) -> SyscallResult {
    let Some(tid) = runtime.current_tid() else {
        return errno_result(EINVAL);
    };
    handle_process_execv_checked_user_syscall::<
        USER_AREAS,
        MAX_ARGS,
        ARG_BYTES,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >(
        runtime, tid, user, path_ptr, argv_ptr, vfs, scratch, stack_bytes, stack_top_va, new_vm,
    )
}

pub struct CurrentExecvDispatchContext<
    'a,
    'v,
    const USER_AREAS: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
> {
    pub runtime: &'a mut crate::trap_runtime::ProcessTrapRuntime<
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >,
    pub user: &'a crate::user_copy::CheckedUserSlice<'a, USER_AREAS>,
    pub vfs: &'a crate::vfs::Vfs<'v>,
    pub scratch: &'a mut [u8],
    pub stack_bytes: &'a mut [u8],
    pub stack_top_va: usize,
    pub new_vm: crate::vm::VmSpace<MAX_AREAS>,
}

pub fn dispatch_current_execv_syscall<
    const USER_AREAS: usize,
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
>(
    syscall: Syscall,
    ctx: CurrentExecvDispatchContext<
        '_,
        '_,
        USER_AREAS,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >,
) -> SyscallResult {
    match syscall {
        Syscall::Execv { path_ptr, argv_ptr, .. } => {
            handle_current_process_execv_checked_user_syscall::<
                USER_AREAS,
                MAX_ARGS,
                ARG_BYTES,
                MAX_PROCS,
                MAX_CHILDREN,
                MAX_AREAS,
                MAX_TASKS,
            >(
                ctx.runtime,
                ctx.user,
                path_ptr,
                argv_ptr,
                ctx.vfs,
                ctx.scratch,
                ctx.stack_bytes,
                ctx.stack_top_va,
                ctx.new_vm,
            )
        }
        _ => SyscallResult::Unsupported,
    }
}

pub struct CurrentExecvRuntimeContext<
    'a,
    'v,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
> {
    pub runtime: &'a mut crate::trap_runtime::ProcessTrapRuntime<
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >,
    pub user_bytes: &'a [u8],
    pub user_base: usize,
    pub vfs: &'a crate::vfs::Vfs<'v>,
    pub scratch: &'a mut [u8],
    pub stack_bytes: &'a mut [u8],
    pub stack_top_va: usize,
    pub new_vm: crate::vm::VmSpace<MAX_AREAS>,
}

pub fn dispatch_current_execv_syscall_from_runtime<
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
>(
    syscall: Syscall,
    ctx: CurrentExecvRuntimeContext<
        '_,
        '_,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >,
) -> SyscallResult {
    let user_vm = match ctx.runtime.current_process_vm() {
        Ok(vm) => vm,
        Err(_) => return errno_result(EINVAL),
    };
    let user = crate::user_copy::CheckedUserSlice::<MAX_AREAS>::new(
        ctx.user_bytes,
        ctx.user_base,
        &user_vm,
    );

    match syscall {
        Syscall::Execv { path_ptr, argv_ptr, .. } => {
            handle_current_process_execv_checked_user_syscall::<
                MAX_AREAS,
                MAX_ARGS,
                ARG_BYTES,
                MAX_PROCS,
                MAX_CHILDREN,
                MAX_AREAS,
                MAX_TASKS,
            >(
                ctx.runtime,
                &user,
                path_ptr,
                argv_ptr,
                ctx.vfs,
                ctx.scratch,
                ctx.stack_bytes,
                ctx.stack_top_va,
                ctx.new_vm,
            )
        }
        _ => SyscallResult::Unsupported,
    }
}

pub fn dispatch_current_execv_frame_from_runtime<
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
>(
    frame: &crate::trap::TrapFrame,
    ctx: CurrentExecvRuntimeContext<
        '_,
        '_,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >,
) -> SyscallResult {
    let Some(syscall) = decode_syscall(frame.a7(), frame.a0(), frame.a1(), frame.a2()) else {
        return SyscallResult::Unsupported;
    };
    dispatch_current_execv_syscall_from_runtime::<
        MAX_ARGS,
        ARG_BYTES,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >(syscall, ctx)
}

pub fn handle_process_execv_syscall(
    _report: crate::trap_runtime::ProcessExecRuntime,
) -> SyscallResult {
    SyscallResult::Continue(0)
}
