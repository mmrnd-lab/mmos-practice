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
