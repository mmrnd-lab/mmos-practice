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

    let child = libc::clone_process();
    if child < 0 {
        let _ = libc::write(2, b"[init] clone failed\n");
        return 1;
    }
    if child == 0 {
        let shell_path = b"/bin/sh\0";
        let shell_argv = [shell_path.as_ptr(), core::ptr::null()];
        let shell_envp = [b"PATH=/bin\0".as_ptr(), core::ptr::null()];
        let _ = libc::execve(shell_path, &shell_argv, &shell_envp);
        let _ = libc::write(2, b"[init-child] execve /bin/sh failed\n");
        return 127;
    }

    let mut status = -1i32;
    let reaped = libc::wait4(child, &mut status);
    if reaped != child {
        let _ = libc::write(2, b"[init] wait4 failed\n");
        return 1;
    }
    if status == 0 {
        let _ = libc::write(1, b"[init] reaped shell pid=2 status=0\n");
        0
    } else {
        let _ = libc::write(2, b"[init] reaped shell pid=2 status=nonzero\n");
        status
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    libc::exit(1)
}
