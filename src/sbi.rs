// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#[cfg(target_arch = "riscv64")]
#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret;
    unsafe {
        core::arch::asm!(
            "ecall",
            inlateout("a0") arg0 => ret,
            in("a1") arg1,
            in("a2") arg2,
            in("a7") which,
        );
    }
    ret
}

#[cfg(target_arch = "riscv64")]
pub fn console_putchar(ch: u8) {
    let _ = sbi_call(1, ch as usize, 0, 0);
}

#[cfg(not(target_arch = "riscv64"))]
pub fn console_putchar(_ch: u8) {}
