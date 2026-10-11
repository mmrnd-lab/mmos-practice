// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::device::{Device, DeviceKind};

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

pub struct PartitionBlockDevice<'a> {
    name: &'a str,
    parent: &'a dyn BlockDevice,
    start_lba: u64,
}

impl<'a> PartitionBlockDevice<'a> {
    pub fn new(name: &'a str, parent: &'a dyn BlockDevice, start_lba: u64) -> Self {
        Self {
            name,
            parent,
            start_lba,
        }
    }
}

impl Device for PartitionBlockDevice<'_> {
    fn name(&self) -> &str {
        self.name
    }

    fn kind(&self) -> DeviceKind {
        DeviceKind::Block
    }
}

impl BlockDevice for PartitionBlockDevice<'_> {
    fn block_size(&self) -> usize {
        self.parent.block_size()
    }

    fn read_block(&self, lba: u64, out: &mut [u8]) -> Result<(), BlockError> {
        self.parent.read_block(self.start_lba + lba, out)
    }
}

pub fn mbr_first_partition_lba(dev: &dyn BlockDevice) -> Result<u64, BlockError> {
    if dev.block_size() != 512 {
        return Err(BlockError::BadBuffer);
    }

    let mut sector = [0u8; 512];
    dev.read_block(0, &mut sector)?;
    if sector[510] != 0x55 || sector[511] != 0xaa {
        return Err(BlockError::Io);
    }

    let p = 446usize;
    let start = u32::from_le_bytes([sector[p + 8], sector[p + 9], sector[p + 10], sector[p + 11]]);
    if start == 0 {
        return Err(BlockError::OutOfRange);
    }

    Ok(start as u64)
}
