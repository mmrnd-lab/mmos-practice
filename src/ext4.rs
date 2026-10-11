// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::block::{BlockDevice, BlockError};
use crate::block_io::read_bytes;

const EXT_SUPERBLOCK_OFFSET: u64 = 1024;
const EXT_SUPERBLOCK_SIZE: usize = 1024;
const EXT_SUPER_MAGIC: u16 = 0xEF53;
const ROOT_INODE_NUMBER: u32 = 2;
const GROUP_DESC_SIZE: usize = 32;
const MAX_BLOCK_SIZE: usize = 1024;
const MAX_INODE_SIZE: usize = 512;
const EXT_S_IFDIR: u16 = 0x4000;
const EXT_S_IFREG: u16 = 0x8000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    Block(BlockError),
    BadSuperblock,
    Corrupt,
    Unsupported,
    NotFound,
    NotDirectory,
    NotFile,
    BufferTooSmall,
}

impl From<BlockError> for FsError {
    fn from(value: BlockError) -> Self {
        Self::Block(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Superblock {
    pub inodes_count: u32,
    pub blocks_count: u32,
    pub log_block_size: u32,
    pub blocks_per_group: u32,
    pub inodes_per_group: u32,
    pub inode_size: u16,
    pub magic: u16,
}

impl Superblock {
    pub fn parse(buf: &[u8]) -> Result<Self, FsError> {
        if buf.len() < EXT_SUPERBLOCK_SIZE {
            return Err(FsError::BadSuperblock);
        }

        let sb = Self {
            inodes_count: le32(buf, 0),
            blocks_count: le32(buf, 4),
            log_block_size: le32(buf, 24),
            blocks_per_group: le32(buf, 32),
            inodes_per_group: le32(buf, 40),
            inode_size: le16(buf, 88),
            magic: le16(buf, 56),
        };

        if sb.magic != EXT_SUPER_MAGIC {
            return Err(FsError::BadSuperblock);
        }

        let inode_size = if sb.inode_size == 0 { 128 } else { sb.inode_size } as usize;
        if inode_size > MAX_INODE_SIZE {
            return Err(FsError::Unsupported);
        }

        let block_size = sb.block_size();
        if block_size == 0 || block_size > MAX_BLOCK_SIZE {
            return Err(FsError::Unsupported);
        }

        Ok(sb)
    }

    pub fn block_size(&self) -> usize {
        1024usize << self.log_block_size
    }

    pub fn inode_size_bytes(&self) -> usize {
        if self.inode_size == 0 { 128 } else { self.inode_size as usize }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GroupDescriptor {
    inode_table_block: u32,
}

impl GroupDescriptor {
    fn parse(buf: &[u8]) -> Result<Self, FsError> {
        if buf.len() < GROUP_DESC_SIZE {
            return Err(FsError::Corrupt);
        }
        Ok(Self { inode_table_block: le32(buf, 8) })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inode {
    number: u32,
    mode: u16,
    size_lo: u32,
    direct_blocks: [u32; 12],
}

impl Inode {
    fn parse(number: u32, buf: &[u8]) -> Result<Self, FsError> {
        if buf.len() < 40 + 15 * 4 {
            return Err(FsError::Corrupt);
        }

        let mut direct_blocks = [0u32; 12];
        let mut i = 0usize;
        while i < direct_blocks.len() {
            direct_blocks[i] = le32(buf, 40 + i * 4);
            i += 1;
        }

        Ok(Self {
            number,
            mode: le16(buf, 0),
            size_lo: le32(buf, 4),
            direct_blocks,
        })
    }

    pub fn number(&self) -> u32 { self.number }
    pub fn size(&self) -> usize { self.size_lo as usize }
    pub fn is_dir(&self) -> bool { self.mode & EXT_S_IFDIR == EXT_S_IFDIR }
    pub fn is_regular_file(&self) -> bool { self.mode & EXT_S_IFREG == EXT_S_IFREG }
}

pub struct Ext4Fs<'a> {
    dev: &'a dyn BlockDevice,
    superblock: Superblock,
}

impl<'a> Ext4Fs<'a> {
    pub fn open(dev: &'a dyn BlockDevice) -> Result<Self, FsError> {
        let mut buf = [0u8; EXT_SUPERBLOCK_SIZE];
        read_bytes(dev, EXT_SUPERBLOCK_OFFSET, &mut buf)?;
        let superblock = Superblock::parse(&buf)?;
        Ok(Self { dev, superblock })
    }

    pub fn superblock(&self) -> &Superblock { &self.superblock }
    pub fn root_inode(&self) -> Result<Inode, FsError> { self.inode_by_number(ROOT_INODE_NUMBER) }

    pub fn inode_by_number(&self, number: u32) -> Result<Inode, FsError> {
        if number == 0 {
            return Err(FsError::NotFound);
        }

        let group = (number - 1) / self.superblock.inodes_per_group;
        let index = (number - 1) % self.superblock.inodes_per_group;
        let gd = self.group_descriptor(group)?;
        let inode_size = self.superblock.inode_size_bytes();
        let inode_offset = gd.inode_table_block as u64 * self.superblock.block_size() as u64
            + index as u64 * inode_size as u64;

        let mut inode_buf = [0u8; MAX_INODE_SIZE];
        read_bytes(self.dev, inode_offset, &mut inode_buf[..inode_size])?;
        Inode::parse(number, &inode_buf[..inode_size])
    }

    pub fn lookup_path(&self, path: &str) -> Result<Inode, FsError> {
        if path == "/" {
            return self.root_inode();
        }
        if !path.starts_with('/') {
            return Err(FsError::NotFound);
        }

        let mut current = self.root_inode()?;
        for component in path.split('/').filter(|part| !part.is_empty()) {
            current = self.lookup_child(&current, component)?;
        }
        Ok(current)
    }

    pub fn lookup_child(&self, dir: &Inode, name: &str) -> Result<Inode, FsError> {
        if !dir.is_dir() {
            return Err(FsError::NotDirectory);
        }

        let block_size = self.superblock.block_size();
        let mut block = [0u8; MAX_BLOCK_SIZE];
        let mut remaining = dir.size();

        for &data_block in &dir.direct_blocks {
            if data_block == 0 || remaining == 0 {
                break;
            }

            self.read_fs_block(data_block as u64, &mut block[..block_size])?;
            let block_limit = core::cmp::min(block_size, remaining);
            let mut offset = 0usize;
            while offset + 8 <= block_limit {
                let inode = le32(&block, offset);
                let rec_len = le16(&block, offset + 4) as usize;
                let name_len = block[offset + 6] as usize;

                if rec_len < 8 || offset + rec_len > block_limit || name_len > rec_len.saturating_sub(8) {
                    return Err(FsError::Corrupt);
                }

                if inode != 0 {
                    let entry_name = &block[offset + 8..offset + 8 + name_len];
                    if entry_name == name.as_bytes() {
                        return self.inode_by_number(inode);
                    }
                }

                offset += rec_len;
            }

            remaining = remaining.saturating_sub(block_limit);
        }

        Err(FsError::NotFound)
    }

    pub fn read_file(&self, inode: &Inode, out: &mut [u8]) -> Result<usize, FsError> {
        if !inode.is_regular_file() {
            return Err(FsError::NotFile);
        }
        if out.len() < inode.size() {
            return Err(FsError::BufferTooSmall);
        }

        let block_size = self.superblock.block_size();
        let mut block = [0u8; MAX_BLOCK_SIZE];
        let mut copied = 0usize;
        let mut remaining = inode.size();

        for &data_block in &inode.direct_blocks {
            if remaining == 0 {
                break;
            }
            if data_block == 0 {
                return Err(FsError::Corrupt);
            }

            self.read_fs_block(data_block as u64, &mut block[..block_size])?;
            let chunk = core::cmp::min(block_size, remaining);
            out[copied..copied + chunk].copy_from_slice(&block[..chunk]);
            copied += chunk;
            remaining -= chunk;
        }

        if remaining != 0 {
            return Err(FsError::Unsupported);
        }

        Ok(copied)
    }

    fn group_descriptor(&self, group: u32) -> Result<GroupDescriptor, FsError> {
        let table_offset = if self.superblock.block_size() == 1024 { 2 * 1024u64 } else { self.superblock.block_size() as u64 };
        let offset = table_offset + group as u64 * GROUP_DESC_SIZE as u64;
        let mut buf = [0u8; GROUP_DESC_SIZE];
        read_bytes(self.dev, offset, &mut buf)?;
        GroupDescriptor::parse(&buf)
    }

    fn read_fs_block(&self, block_number: u64, out: &mut [u8]) -> Result<(), FsError> {
        let offset = block_number * self.superblock.block_size() as u64;
        read_bytes(self.dev, offset, out)?;
        Ok(())
    }
}

fn le16(buf: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([buf[offset], buf[offset + 1]])
}

fn le32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([buf[offset], buf[offset + 1], buf[offset + 2], buf[offset + 3]])
}
