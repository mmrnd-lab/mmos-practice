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

unsafe extern "C" {
    fn main(argc: usize, argv: *const *const u8, envp: *const *const u8) -> i32;
    fn __mmos_libc_start_main(
        main: unsafe extern "C" fn(usize, *const *const u8, *const *const u8) -> i32,
        argc: usize,
        argv: *const *const u8,
        envp: *const *const u8,
    ) -> !;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn __mmos_user_crt_start(initial_sp: *const usize) -> ! {
    let argc = unsafe { initial_sp.read() };
    let argv = unsafe { initial_sp.add(1) as *const *const u8 };
    let envp = unsafe { initial_sp.add(1 + argc + 1) as *const *const u8 };
    unsafe { __mmos_libc_start_main(main, argc, argv, envp) }
}
