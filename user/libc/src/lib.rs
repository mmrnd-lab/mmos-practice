#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0


const SYS_OPENAT: usize = 56;
const SYS_READ: usize = 63;
const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const SYS_CLONE: usize = 220;
const SYS_EXECVE: usize = 221;
const SYS_WAIT4: usize = 260;
const AT_FDCWD: usize = (-100isize) as usize;
const O_RDONLY: usize = 0;
const O_WRONLY: usize = 1;
const STARTUP_CSTR_MAX: usize = 256;

pub type MainFn =
    unsafe extern "C" fn(argc: usize, argv: *const *const u8, envp: *const *const u8) -> i32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __mmos_libc_start_main(
    main: MainFn,
    argc: usize,
    argv: *const *const u8,
    envp: *const *const u8,
) -> ! {
    let code = unsafe { main(argc, argv, envp) };
    exit(code)
}

#[inline(always)]
fn syscall1(nr: usize, a0: usize) -> usize {
    let ret;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") a0 => ret,
            in("a7") nr,
        );
    }
    ret
}

#[inline(always)]
fn syscall3(nr: usize, a0: usize, a1: usize, a2: usize) -> usize {
    let ret;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") a0 => ret,
            in("a1") a1,
            in("a2") a2,
            in("a7") nr,
        );
    }
    ret
}

pub fn openat(dirfd: usize, path: &[u8], flags: usize) -> isize {
    syscall3(SYS_OPENAT, dirfd, path.as_ptr() as usize, flags) as isize
}

pub fn write(fd: usize, bytes: &[u8]) -> isize {
    syscall3(SYS_WRITE, fd, bytes.as_ptr() as usize, bytes.len()) as isize
}

pub fn read(fd: usize, bytes: &mut [u8]) -> isize {
    syscall3(SYS_READ, fd, bytes.as_mut_ptr() as usize, bytes.len()) as isize
}

pub fn clone_process() -> isize {
    syscall3(SYS_CLONE, 0, 0, 0) as isize
}

pub fn execve(path: &[u8], argv: &[*const u8], envp: &[*const u8]) -> isize {
    syscall3(
        SYS_EXECVE,
        path.as_ptr() as usize,
        argv.as_ptr() as usize,
        envp.as_ptr() as usize,
    ) as isize
}

pub fn wait4(pid: isize, status: &mut i32) -> isize {
    syscall3(SYS_WAIT4, pid as usize, status as *mut i32 as usize, 0) as isize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_startup_cstr(fd: usize, ptr: *const u8) -> isize {
    if ptr.is_null() {
        return 0;
    }

    let mut len = 0usize;
    while len < STARTUP_CSTR_MAX && unsafe { ptr.add(len).read() } != 0 {
        len += 1;
    }
    let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
    write(fd, bytes)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_env_value(
    fd: usize,
    envp: *const *const u8,
    key: *const u8,
) -> isize {
    if envp.is_null() || key.is_null() {
        return 0;
    }

    let key_len = startup_cstr_len(key);
    let mut idx = 0usize;
    while idx < STARTUP_CSTR_MAX {
        let entry = unsafe { envp.add(idx).read() };
        if entry.is_null() {
            return 0;
        }

        if env_matches_key(entry, key, key_len) {
            return unsafe { write_startup_cstr(fd, entry.add(key_len + 1)) };
        }
        idx += 1;
    }
    0
}

fn startup_cstr_len(ptr: *const u8) -> usize {
    let mut len = 0usize;
    while len < STARTUP_CSTR_MAX && unsafe { ptr.add(len).read() } != 0 {
        len += 1;
    }
    len
}

fn env_matches_key(entry: *const u8, key: *const u8, key_len: usize) -> bool {
    let mut idx = 0usize;
    while idx < key_len {
        if unsafe { entry.add(idx).read() } != unsafe { key.add(idx).read() } {
            return false;
        }
        idx += 1;
    }
    let separator = unsafe { entry.add(key_len).read() };
    separator == b'='
}

pub fn init_stdio_console() {
    let console = b"/dev/console\0";
    let _ = openat(AT_FDCWD, console, O_RDONLY);
    let _ = openat(AT_FDCWD, console, O_WRONLY);
    let _ = openat(AT_FDCWD, console, O_WRONLY);
}

pub fn exit(code: i32) -> ! {
    let _ = syscall1(SYS_EXIT, code as usize);
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}
