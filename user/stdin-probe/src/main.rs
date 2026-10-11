#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use mmos_user_crt as _;
use mmos_user_libc as libc;

#[unsafe(no_mangle)]
pub extern "C" fn main(_: usize, _: *const *const u8, _: *const *const u8) -> i32 {
    libc::init_stdio_console();
    let _ = libc::write(1, b"[stdin-probe] type one byte: ");

    let mut byte = [0u8; 1];
    if libc::read(0, &mut byte) != 1 {
        let _ = libc::write(1, b"\n[stdin-probe] read failed\n");
        return 1;
    }

    let _ = libc::write(1, b"\n[stdin-probe] received=");
    let _ = libc::write(1, &byte);
    let _ = libc::write(1, b"\n");
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    libc::exit(1)
}
