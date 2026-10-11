// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::block::BlockDevice;
use crate::ext4::{Ext4Fs, FsError};

pub struct Vfs<'a> {
    ext4: Ext4Fs<'a>,
}

impl<'a> Vfs<'a> {
    pub fn mount_ext4(dev: &'a dyn BlockDevice) -> Result<Self, FsError> {
        Ok(Self {
            ext4: Ext4Fs::open(dev)?,
        })
    }

    pub fn read_all(&self, path: &str, out: &mut [u8]) -> Result<usize, FsError> {
        let inode = self.ext4.lookup_path(path)?;
        self.ext4.read_file(&inode, out)
    }

    pub fn ext4(&self) -> &Ext4Fs<'a> {
        &self.ext4
    }
}
