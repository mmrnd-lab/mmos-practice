#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use mmos_user_crt as crt;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    crt::init_stdio_console();
    let msg = b"[user-init] hello from user app\n";
    let _ = crt::write(1, msg);
    crt::exit(0)
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    crt::exit(1)
}
