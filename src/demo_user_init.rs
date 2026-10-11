// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

pub const DEMO_EXT4_IMAGE_SIZE: usize = 16 * 1024;
pub const DEMO_INIT_ELF_SIZE: usize = 0x1c0;
pub const DEMO_SHARED_OBJECT_SIZE: usize = DEMO_INIT_ELF_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoBuildError {
    ImageTooSmall,
    ElfTooSmall,
}

pub fn build_demo_init_elf(out: &mut [u8]) -> Result<usize, DemoBuildError> {
    const BASE_VADDR: usize = 0x4000_0000;
    const SEG_OFF: usize = 0x100;
    const PATH_OFF: usize = 0x170;
    const MSG_OFF: usize = 0x180;
    const PATH: &[u8] = b"/dev/console\0";
    const MSG: &[u8] = b"[user-init] hello from user app\n";

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
    write_le64(out, 24, BASE_VADDR as u64);
    write_le64(out, 32, 64);
    write_le16(out, 52, 64);
    write_le16(out, 54, 56);
    write_le16(out, 56, 1);

    let ph = 64usize;
    write_le32(out, ph, 1);
    write_le32(out, ph + 4, 5);
    write_le64(out, ph + 8, SEG_OFF as u64);
    write_le64(out, ph + 16, BASE_VADDR as u64);
    let file_size = (MSG_OFF + MSG.len()) - SEG_OFF;
    write_le64(out, ph + 32, file_size as u64);
    write_le64(out, ph + 40, file_size as u64);
    write_le64(out, ph + 48, 0x1000);

    let code = &mut out[SEG_OFF..MSG_OFF];
    code.fill(0);
    // init owns the first stdio contract: open /dev/console as fd 0, 1, and 2.
    write_le32(code, 0x00, encode_auipc(11, 0));
    write_le32(code, 0x04, encode_addi(11, 11, (PATH_OFF - SEG_OFF) as i32));
    write_le32(code, 0x08, encode_addi(10, 0, -100));
    write_le32(code, 0x0c, encode_addi(12, 0, 0));
    write_le32(code, 0x10, encode_addi(17, 0, 56));
    write_le32(code, 0x14, 0x0000_0073); // ecall
    write_le32(code, 0x18, encode_auipc(11, 0));
    write_le32(code, 0x1c, encode_addi(11, 11, (PATH_OFF - (SEG_OFF + 0x18)) as i32));
    write_le32(code, 0x20, encode_addi(10, 0, -100));
    write_le32(code, 0x24, encode_addi(12, 0, 1));
    write_le32(code, 0x28, encode_addi(17, 0, 56));
    write_le32(code, 0x2c, 0x0000_0073); // ecall
    write_le32(code, 0x30, encode_auipc(11, 0));
    write_le32(code, 0x34, encode_addi(11, 11, (PATH_OFF - (SEG_OFF + 0x30)) as i32));
    write_le32(code, 0x38, encode_addi(10, 0, -100));
    write_le32(code, 0x3c, encode_addi(12, 0, 1));
    write_le32(code, 0x40, encode_addi(17, 0, 56));
    write_le32(code, 0x44, 0x0000_0073); // ecall
    // a1 = &MSG (PC-relative, avoids RV64 sign-extension pitfalls)
    write_le32(code, 0x48, encode_auipc(11, 0));
    write_le32(code, 0x4c, encode_addi(11, 11, (MSG_OFF - (SEG_OFF + 0x48)) as i32));
    // write(1, msg, msg_len)
    write_le32(code, 0x50, encode_addi(10, 0, 1));
    write_le32(code, 0x54, encode_addi(12, 0, MSG.len() as i32));
    write_le32(code, 0x58, encode_addi(17, 0, 64));
    write_le32(code, 0x5c, 0x0000_0073); // ecall
                                         // exit(0)
    write_le32(code, 0x60, encode_addi(10, 0, 0));
    write_le32(code, 0x64, encode_addi(17, 0, 93));
    write_le32(code, 0x68, 0x0000_0073); // ecall
                                         // safety loop if exit path is not wired
    write_le32(code, 0x6c, 0x1050_0073); // wfi
    write_le32(code, 0x70, 0x0000_006f); // j .

    out[PATH_OFF..PATH_OFF + PATH.len()].copy_from_slice(PATH);
    out[MSG_OFF..MSG_OFF + MSG.len()].copy_from_slice(MSG);
    Ok(DEMO_INIT_ELF_SIZE)
}

pub fn build_demo_ext4_image(image: &mut [u8], elf: &[u8]) -> Result<(), DemoBuildError> {
    build_demo_ext4_image_with_shared_object(image, elf, &[])
}

pub fn build_demo_shared_object(out: &mut [u8]) -> Result<usize, DemoBuildError> {
    let len = build_demo_init_elf(out)?;
    write_le16(out, 16, 3);
    Ok(len)
}

pub fn build_demo_ext4_image_with_shared_object(
    image: &mut [u8],
    elf: &[u8],
    shared_object: &[u8],
) -> Result<(), DemoBuildError> {
    if image.len() < DEMO_EXT4_IMAGE_SIZE {
        return Err(DemoBuildError::ImageTooSmall);
    }
    image[..DEMO_EXT4_IMAGE_SIZE].fill(0);
    write_superblock(image);
    write_group_descriptor(image);
    write_inode(image, 2, 0x4000, 1024, 10);
    write_inode(image, 12, 0x4000, 1024, 11);
    write_inode(image, 13, 0x8000, elf.len() as u32, 12);
    if !shared_object.is_empty() {
        write_inode(image, 14, 0x4000, 1024, 13);
        write_inode(image, 15, 0x8000, shared_object.len() as u32, 14);
    }
    write_root_dir(image, !shared_object.is_empty());
    write_sbin_dir(image);
    if !shared_object.is_empty() {
        write_lib_dir(image);
        let so_start = 14 * 1024;
        let so_end = so_start + shared_object.len();
        image[so_start..so_end].copy_from_slice(shared_object);
    }
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

fn write_root_dir(image: &mut [u8], include_lib: bool) {
    let block = &mut image[10 * 1024..11 * 1024];
    write_dir_entry(block, 0, 2, 12, b".", 2);
    write_dir_entry(block, 12, 2, 12, b"..", 2);
    if include_lib {
        write_dir_entry(block, 24, 12, 12, b"sbin", 2);
        write_dir_entry(block, 36, 14, (1024 - 36) as u16, b"lib", 2);
    } else {
        write_dir_entry(block, 24, 12, (1024 - 24) as u16, b"sbin", 2);
    }
}

fn write_sbin_dir(image: &mut [u8]) {
    let block = &mut image[11 * 1024..12 * 1024];
    write_dir_entry(block, 0, 12, 12, b".", 2);
    write_dir_entry(block, 12, 2, 12, b"..", 2);
    write_dir_entry(block, 24, 13, (1024 - 24) as u16, b"init", 1);
}

fn write_lib_dir(image: &mut [u8]) {
    let block = &mut image[13 * 1024..14 * 1024];
    write_dir_entry(block, 0, 14, 12, b".", 2);
    write_dir_entry(block, 12, 2, 12, b"..", 2);
    write_dir_entry(block, 24, 15, (1024 - 24) as u16, b"libmmos_user_libc.so", 1);
}

fn write_dir_entry(
    block: &mut [u8],
    offset: usize,
    inode: u32,
    rec_len: u16,
    name: &[u8],
    file_type: u8,
) {
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

fn encode_addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    let imm12 = (imm as u32) & 0x0fff;
    (imm12 << 20) | (rs1 << 15) | (rd << 7) | 0x13
}

fn encode_auipc(rd: u32, imm20: u32) -> u32 {
    (imm20 << 12) | (rd << 7) | 0x17
}
