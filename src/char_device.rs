// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::device::Device;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharError {
    WouldBlock,
    Io,
    BadBuffer,
    Unsupported,
}

#[allow(dead_code)]
pub trait CharacterDevice: Device {
    fn read(&self, out: &mut [u8]) -> Result<usize, CharError>;
    fn write(&self, bytes: &[u8]) -> Result<usize, CharError>;
}
