#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use mmos_user_crt as _;
use mmos_user_libc as libc;

const LINE_CAPACITY: usize = 128;

#[unsafe(no_mangle)]
pub extern "C" fn main(_: usize, _: *const *const u8, _: *const *const u8) -> i32 {
    libc::init_stdio_console();
    let _ = libc::write(1, b"[shell] MMOS shell ready\n");

    let mut line = [0u8; LINE_CAPACITY];
    loop {
        let _ = libc::write(1, b"mmos$ ");
        let len = match read_line(&mut line) {
            Some(len) => len,
            None => return 1,
        };

        let command = trim_ascii_spaces(&line[..len]);
        if command.is_empty() {
            continue;
        }
        if command == b"exit" {
            let _ = libc::write(1, b"[shell] exit\n");
            return 0;
        }
        if command == b"help" {
            let _ = libc::write(1, b"builtins: help echo exit\n");
            continue;
        }
        if command == b"echo" {
            let _ = libc::write(1, b"\n");
            continue;
        }
        if let Some(rest) = command.strip_prefix(b"echo ") {
            let _ = libc::write(1, rest);
            let _ = libc::write(1, b"\n");
            continue;
        }

        let _ = libc::write(1, b"unknown command: ");
        let _ = libc::write(1, command);
        let _ = libc::write(1, b"\n");
    }
}

fn read_line(line: &mut [u8; LINE_CAPACITY]) -> Option<usize> {
    let mut len = 0usize;
    loop {
        let mut byte = [0u8; 1];
        if libc::read(0, &mut byte) != 1 {
            return None;
        }

        match byte[0] {
            b'\r' | b'\n' => {
                let _ = libc::write(1, b"\n");
                return Some(len);
            }
            0x08 | 0x7f if len > 0 => {
                len -= 1;
                let _ = libc::write(1, b"\x08 \x08");
            }
            byte if !byte.is_ascii_control() && len < line.len() => {
                line[len] = byte;
                len += 1;
                let _ = libc::write(1, &byte.to_ne_bytes());
            }
            _ => {}
        }
    }
}

fn trim_ascii_spaces(mut bytes: &[u8]) -> &[u8] {
    while bytes.first() == Some(&b' ') {
        bytes = &bytes[1..];
    }
    while bytes.last() == Some(&b' ') {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    libc::exit(1)
}
