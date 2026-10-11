// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::char_device::{CharError, CharacterDevice};
use crate::device::{Device, DeviceKind};

#[cfg(feature = "console-mmio")]
pub const UART0_BASE: usize = 0x1000_0000;

#[cfg(not(feature = "console-mmio"))]
pub const UART0_BASE: usize = 0;

pub struct SerialDevice {
    name: &'static str,
    base: usize,
}

impl SerialDevice {
    pub const fn new(name: &'static str, base: usize) -> Self {
        Self { name, base }
    }

    #[allow(dead_code)]
    pub const fn base(&self) -> usize {
        self.base
    }
}

impl Device for SerialDevice {
    fn name(&self) -> &str {
        self.name
    }

    fn kind(&self) -> DeviceKind {
        DeviceKind::Char
    }
}

impl CharacterDevice for SerialDevice {
    fn read(&self, _out: &mut [u8]) -> Result<usize, CharError> {
        Err(CharError::Unsupported)
    }

    fn write(&self, bytes: &[u8]) -> Result<usize, CharError> {
        if bytes.is_empty() {
            return Ok(0);
        }
        crate::uart::put_bytes_at(self.base, bytes);
        Ok(bytes.len())
    }
}
