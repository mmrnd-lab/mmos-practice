// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::block::{BlockDevice, BlockError};

const MAX_BLOCK_SIZE: usize = 4096;

pub fn read_bytes(dev: &dyn BlockDevice, offset: u64, out: &mut [u8]) -> Result<(), BlockError> {
    if out.is_empty() {
        return Ok(());
    }

    let block_size = dev.block_size();
    if block_size == 0 || block_size > MAX_BLOCK_SIZE {
        return Err(BlockError::BadBuffer);
    }

    let mut scratch = [0u8; MAX_BLOCK_SIZE];
    let block_buf = &mut scratch[..block_size];
    let mut copied = 0usize;

    while copied < out.len() {
        let absolute = offset as usize + copied;
        let lba = absolute / block_size;
        let in_block_off = absolute % block_size;
        let chunk = core::cmp::min(block_size - in_block_off, out.len() - copied);

        dev.read_block(lba as u64, block_buf)?;
        out[copied..copied + chunk].copy_from_slice(&block_buf[in_block_off..in_block_off + chunk]);
        copied += chunk;
    }

    Ok(())
}
