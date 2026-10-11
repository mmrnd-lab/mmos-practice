#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

mod rt;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    rt::init_stdio_console();
    let msg = b"[user-init] hello from user app\n";
    let _ = rt::write(1, msg);
    rt::exit(0)
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    rt::exit(1)
}
