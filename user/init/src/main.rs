#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use mmos_user_crt as _;
use mmos_user_libc as libc;

#[unsafe(no_mangle)]
pub extern "C" fn main(argc: usize, argv: *const *const u8, envp: *const *const u8) -> i32 {
    libc::init_stdio_console();
    let msg = b"[user-init] hello from user app\n";
    let _ = libc::write(1, msg);
    let msg = b"[user-init] main reached through crt entry\n";
    let _ = libc::write(1, msg);
    if argc > 0 {
        let msg = b"[user-init] argv0=";
        let _ = libc::write(1, msg);
        let _ = unsafe { libc::write_startup_cstr(1, argv.read()) };
        let _ = libc::write(1, b"\n");
    }
    let first_env = unsafe { envp.read() };
    if !first_env.is_null() {
        let msg = b"[user-init] env0=";
        let _ = libc::write(1, msg);
        let _ = unsafe { libc::write_startup_cstr(1, first_env) };
        let _ = libc::write(1, b"\n");
    }
    let msg = b"[user-init] getenv PATH=";
    let _ = libc::write(1, msg);
    let _ = unsafe { libc::write_env_value(1, envp, b"PATH\0".as_ptr()) };
    let _ = libc::write(1, b"\n");
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    libc::exit(1)
}
