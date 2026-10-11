// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::device::Device;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockError {
    OutOfRange,
    Io,
    BadBuffer,
}

pub trait BlockDevice: Device {
    fn block_size(&self) -> usize;
    fn read_block(&self, lba: u64, out: &mut [u8]) -> Result<(), BlockError>;
}

