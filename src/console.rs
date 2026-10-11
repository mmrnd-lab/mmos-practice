// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::char_device::{CharError, CharacterDevice};
use crate::serial::{SerialDevice, UART0_BASE};
use crate::device::{Device, DeviceKind};

static SERIAL0: SerialDevice = SerialDevice::new("ttyS0", UART0_BASE);

pub fn device() -> &'static dyn CharacterDevice {
    &SERIAL0
}

pub fn write(bytes: &[u8]) -> Result<usize, CharError> {
    device().write(bytes)
}

pub fn put_bytes(bytes: &[u8]) {
    let _ = write(bytes);
}

pub fn puts(s: &str) {
    put_bytes(s.as_bytes());
}


pub struct BufferedConsoleDevice {
    name: &'static str,
    input: &'static [u8],
    cursor: AtomicUsize,
}

impl BufferedConsoleDevice {
    pub const fn new(name: &'static str, input: &'static [u8]) -> Self {
        Self {
            name,
            input,
            cursor: AtomicUsize::new(0),
        }
    }

    pub fn reset(&self) {
        self.cursor.store(0, Ordering::SeqCst);
    }
}

impl Device for BufferedConsoleDevice {
    fn name(&self) -> &str {
        self.name
    }

    fn kind(&self) -> DeviceKind {
        DeviceKind::Char
    }
}

impl CharacterDevice for BufferedConsoleDevice {
    fn read(&self, out: &mut [u8]) -> Result<usize, CharError> {
        if out.is_empty() {
            return Ok(0);
        }

        let start = self.cursor.load(Ordering::SeqCst);
        if start >= self.input.len() {
            return Err(CharError::WouldBlock);
        }
        let n = core::cmp::min(out.len(), self.input.len() - start);
        out[..n].copy_from_slice(&self.input[start..start + n]);
        self.cursor.store(start + n, Ordering::SeqCst);
        Ok(n)
    }

    fn write(&self, bytes: &[u8]) -> Result<usize, CharError> {
        device().write(bytes)
    }
}

static READBACK_CONSOLE: BufferedConsoleDevice =
    BufferedConsoleDevice::new("ttyS0-readback", b"runtime-read");

pub fn readback_device() -> &'static BufferedConsoleDevice {
    &READBACK_CONSOLE
}
