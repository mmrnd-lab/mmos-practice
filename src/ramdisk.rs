// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::block::{BlockDevice, BlockError};
use crate::device::{Device, DeviceKind};

#[derive(Debug, Clone, Copy)]
pub struct RamDisk<'a> {
    name: &'a str,
    storage: &'a [u8],
    block_size: usize,
}

impl<'a> RamDisk<'a> {
    pub fn new(name: &'a str, storage: &'a [u8], block_size: usize) -> Self {
        debug_assert!(block_size != 0);
        debug_assert_eq!(storage.len() % block_size, 0);
        Self {
            name,
            storage,
            block_size,
        }
    }

    pub fn total_blocks(&self) -> usize {
        self.storage.len() / self.block_size
    }
}

impl Device for RamDisk<'_> {
    fn name(&self) -> &str {
        self.name
    }

    fn kind(&self) -> DeviceKind {
        DeviceKind::Block
    }
}

impl BlockDevice for RamDisk<'_> {
    fn block_size(&self) -> usize {
        self.block_size
    }

    fn read_block(&self, lba: u64, out: &mut [u8]) -> Result<(), BlockError> {
        if out.len() != self.block_size {
            return Err(BlockError::BadBuffer);
        }

        let start = lba as usize * self.block_size;
        let end = start + self.block_size;
        let src = self.storage.get(start..end).ok_or(BlockError::OutOfRange)?;
        out.copy_from_slice(src);
        Ok(())
    }
}
