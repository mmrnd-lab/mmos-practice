// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

pub const DEMO_EXT4_IMAGE_SIZE: usize = 16 * 1024;
pub const DEMO_INIT_ELF_SIZE: usize = 0x104;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoBuildError {
    ImageTooSmall,
    ElfTooSmall,
}

pub fn build_demo_init_elf(out: &mut [u8]) -> Result<usize, DemoBuildError> {
    if out.len() < DEMO_INIT_ELF_SIZE {
        return Err(DemoBuildError::ElfTooSmall);
    }
    out[..DEMO_INIT_ELF_SIZE].fill(0);
    out[0..4].copy_from_slice(b"\x7fELF");
    out[4] = 2;
    out[5] = 1;
    out[6] = 1;
    write_le16(out, 16, 2);
    write_le16(out, 18, 243);
    write_le32(out, 20, 1);
    write_le64(out, 24, 0x8040_0000);
    write_le64(out, 32, 64);
    write_le16(out, 52, 64);
    write_le16(out, 54, 56);
    write_le16(out, 56, 1);

    let ph = 64usize;
    write_le32(out, ph, 1);
    write_le32(out, ph + 4, 5);
    write_le64(out, ph + 8, 0x100);
    write_le64(out, ph + 16, 0x8040_0000);
    write_le64(out, ph + 32, 4);
    write_le64(out, ph + 40, 8);
    write_le64(out, ph + 48, 0x1000);
    out[0x100..0x104].copy_from_slice(&[0x13, 0x00, 0x00, 0x00]);
    Ok(DEMO_INIT_ELF_SIZE)
}

pub fn build_demo_ext4_image(image: &mut [u8], elf: &[u8]) -> Result<(), DemoBuildError> {
    if image.len() < DEMO_EXT4_IMAGE_SIZE {
        return Err(DemoBuildError::ImageTooSmall);
    }
    image[..DEMO_EXT4_IMAGE_SIZE].fill(0);
    write_superblock(image);
    write_group_descriptor(image);
    write_inode(image, 2, 0x4000, 1024, 10);
    write_inode(image, 12, 0x4000, 1024, 11);
    write_inode(image, 13, 0x8000, elf.len() as u32, 12);
    write_root_dir(image);
    write_sbin_dir(image);
    let start = 12 * 1024;
    let end = start + elf.len();
    image[start..end].copy_from_slice(elf);
    Ok(())
}

fn write_superblock(image: &mut [u8]) {
    let sb = &mut image[1024..2048];
    write_le32(sb, 0, 32);
    write_le32(sb, 4, 64);
    write_le32(sb, 24, 0);
    write_le32(sb, 32, 16);
    write_le32(sb, 40, 32);
    write_le16(sb, 56, 0xEF53);
    write_le16(sb, 88, 128);
}

fn write_group_descriptor(image: &mut [u8]) {
    let gd = &mut image[2048..2080];
    write_le32(gd, 8, 5);
}

fn write_inode(image: &mut [u8], inode_number: u32, mode: u16, size: u32, first_block: u32) {
    let inode_size = 128usize;
    let inode_table_offset = 5 * 1024usize;
    let index = (inode_number as usize).saturating_sub(1);
    let base = inode_table_offset + index * inode_size;
    write_le16(&mut image[base..base + inode_size], 0, mode);
    write_le32(&mut image[base..base + inode_size], 4, size);
    write_le32(&mut image[base..base + inode_size], 40, first_block);
}

fn write_root_dir(image: &mut [u8]) {
    let block = &mut image[10 * 1024..11 * 1024];
    write_dir_entry(block, 0, 2, 12, b".", 2);
    write_dir_entry(block, 12, 2, 12, b"..", 2);
    write_dir_entry(block, 24, 12, (1024 - 24) as u16, b"sbin", 2);
}

fn write_sbin_dir(image: &mut [u8]) {
    let block = &mut image[11 * 1024..12 * 1024];
    write_dir_entry(block, 0, 12, 12, b".", 2);
    write_dir_entry(block, 12, 2, 12, b"..", 2);
    write_dir_entry(block, 24, 13, (1024 - 24) as u16, b"init", 1);
}

fn write_dir_entry(block: &mut [u8], offset: usize, inode: u32, rec_len: u16, name: &[u8], file_type: u8) {
    write_le32(block, offset, inode);
    write_le16(block, offset + 4, rec_len);
    block[offset + 6] = name.len() as u8;
    block[offset + 7] = file_type;
    block[offset + 8..offset + 8 + name.len()].copy_from_slice(name);
}

fn write_le16(buf: &mut [u8], offset: usize, value: u16) {
    buf[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_le32(buf: &mut [u8], offset: usize, value: u32) {
    buf[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_le64(buf: &mut [u8], offset: usize, value: u64) {
    buf[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
