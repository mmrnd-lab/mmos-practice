// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::char_device::{CharError, CharacterDevice};
use crate::serial::{SerialDevice, UART0_BASE};

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
