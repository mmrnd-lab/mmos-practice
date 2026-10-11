#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use mmos_user_crt as crt;

#[unsafe(no_mangle)]
pub extern "C" fn main(
    _argc: usize,
    _argv: *const *const u8,
    _envp: *const *const u8,
) -> i32 {
    crt::init_stdio_console();
    let msg = b"[user-init] hello from user app\n";
    let _ = crt::write(1, msg);
    let msg = b"[user-init] main reached through crt entry\n";
    let _ = crt::write(1, msg);
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    crt::exit(1)
}
