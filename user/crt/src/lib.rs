#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0


core::arch::global_asm!(
    r#"
    .section .text.entry,"ax"
    .globl _start
_start:
    mv a0, sp
    call __mmos_user_crt_start
1:
    wfi
    j 1b
"#
);

const SYS_OPENAT: usize = 56;
const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;
const AT_FDCWD: usize = (-100isize) as usize;
const O_RDONLY: usize = 0;
const O_WRONLY: usize = 1;
const STARTUP_CSTR_MAX: usize = 256;

unsafe extern "C" {
    fn main(argc: usize, argv: *const *const u8, envp: *const *const u8) -> i32;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn __mmos_user_crt_start(initial_sp: *const usize) -> ! {
    let argc = unsafe { initial_sp.read() };
    let argv = unsafe { initial_sp.add(1) as *const *const u8 };
    let envp = unsafe { initial_sp.add(1 + argc + 1) as *const *const u8 };
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
