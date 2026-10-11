#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

const SYS_WRITE: usize = 64;
const SYS_EXIT: usize = 93;

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

fn exit(code: i32) -> ! {
    let _ = syscall1(SYS_EXIT, code as usize);
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let msg = b"[user-init] hello from user app\n";
    let _ = syscall3(SYS_WRITE, 1, msg.as_ptr() as usize, msg.len());
    exit(0)
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(1)
}
