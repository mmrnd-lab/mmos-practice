#![cfg_attr(not(test), no_std)]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0


pub mod block;
pub mod ramdisk;
pub mod block_io;
pub mod device;
pub mod ext4;
pub mod irq_guard;
pub mod kheap;
pub mod page_frame_allocator;
pub mod syscall;
pub mod sv39;
pub mod trap;
pub mod user_init;
pub mod vfs;

pub fn align_up(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (value + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::align_up;
    use super::block::{BlockDevice, BlockError};
    use super::ramdisk::RamDisk;
    use super::block_io::read_bytes;
    use super::device::{Device, DeviceKind};
    use super::ext4::{Ext4Fs, FsError};
    use super::kheap::init_kheap;
    use super::page_frame_allocator::PageFrameAllocator;
    use super::syscall::{decode_syscall, handle_syscall, Syscall, SyscallResult, SYS_EXIT, SYS_WRITE};
    use super::sv39::{make_satp, PageTableEntry, RootPageTable, PTE_A, PTE_D, PTE_R, PTE_W, PTE_X};
    use super::trap::{handle_trap, TrapFrame, TrapOutcome, SCAUSE_USER_ECALL};
    use super::user_init::{build_user_init_context, run_user_init_demo, SSTATUS_SPP};
    use super::vfs::Vfs;

    #[test]
    fn align_up_keeps_aligned_values() {
        assert_eq!(align_up(0x1000, 0x1000), 0x1000);
    }

    #[test]
    fn align_up_rounds_to_next_boundary() {
        assert_eq!(align_up(0x1001, 0x1000), 0x2000);
    }

    #[test]
    fn kheap_region_end_is_calculated() {
        let region = init_kheap(0x9000_0000, 0x10000);
        assert_eq!(region.end(), 0x9001_0000);
    }

    #[test]
    fn ramdisk_is_registered_as_block_kind() {
        let storage = [0u8; 16];
        let dev = RamDisk::new("ramblk0", &storage, 8);
        assert_eq!(dev.name(), "ramblk0");
        assert_eq!(dev.kind(), DeviceKind::Block);
        assert_eq!(dev.total_blocks(), 2);
    }

    #[test]
    fn block_read_single_lba() {
        let storage: [u8; 16] = [
            0, 1, 2, 3, 4, 5, 6, 7,
            8, 9, 10, 11, 12, 13, 14, 15,
        ];
        let dev = RamDisk::new("ramblk0", &storage, 8);
        let mut out = [0u8; 8];
        dev.read_block(1, &mut out).unwrap();
        assert_eq!(out, [8, 9, 10, 11, 12, 13, 14, 15]);
    }

    #[test]
    fn block_read_rejects_wrong_sized_buffer() {
        let storage = [0u8; 16];
        let dev = RamDisk::new("ramblk0", &storage, 8);
        let mut out = [0u8; 4];
        assert_eq!(dev.read_block(0, &mut out), Err(BlockError::BadBuffer));
    }

    #[test]
    fn block_read_across_boundary() {
        let storage: [u8; 16] = [
            b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H',
            b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
        ];
        let dev = RamDisk::new("ramblk0", &storage, 8);
        let mut out = [0u8; 6];
        read_bytes(&dev, 6, &mut out).unwrap();
        assert_eq!(&out, b"GHIJKL");
    }

    #[test]
    fn block_read_reports_out_of_range() {
        let storage = [0u8; 16];
        let dev = RamDisk::new("ramblk0", &storage, 8);
        let mut out = [0u8; 4];
        assert_eq!(read_bytes(&dev, 14, &mut out), Err(BlockError::OutOfRange));
    }

    #[test]
    fn ext4_superblock_magic_is_valid() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let fs = Ext4Fs::open(&dev).unwrap();
        assert_eq!(fs.superblock().magic, 0xEF53);
        assert_eq!(fs.superblock().block_size(), 1024);
    }

    #[test]
    fn ext4_inode_by_number_reads_root_directory() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let fs = Ext4Fs::open(&dev).unwrap();
        let root = fs.inode_by_number(2).unwrap();
        assert!(root.is_dir());
        assert_eq!(root.size(), 1024);
    }

    #[test]
    fn ext4_lookup_path_finds_sbin_init() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let fs = Ext4Fs::open(&dev).unwrap();
        let inode = fs.lookup_path("/sbin/init").unwrap();
        assert!(inode.is_regular_file());
        assert_eq!(inode.number(), 13);
        assert_eq!(inode.size(), 8);
    }

    #[test]
    fn ext4_read_file_bytes_returns_init_elf() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let fs = Ext4Fs::open(&dev).unwrap();
        let inode = fs.lookup_path("/sbin/init").unwrap();
        let mut out = [0u8; 8];
        let copied = fs.read_file(&inode, &mut out).unwrap();
        assert_eq!(copied, 8);
        assert_eq!(&out, b"\x7fELFinit");
    }

    #[test]
    fn vfs_read_all_returns_init_elf_bytes() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut out = [0u8; 8];
        let copied = vfs.read_all("/sbin/init", &mut out).unwrap();
        assert_eq!(copied, 8);
        assert_eq!(&out, b"\x7fELFinit");
    }

    #[test]
    fn vfs_read_all_fails_on_directory() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut out = [0u8; 16];
        assert_eq!(vfs.read_all("/sbin", &mut out), Err(FsError::NotFile));
    }

    #[test]
    fn page_frame_allocator_allocates_sequential_frames() {
        let mut pfa = PageFrameAllocator::new(0x8020_0000, 0x8020_3000, 0x1000);
        assert_eq!(pfa.alloc(), Some(0x8020_0000));
        assert_eq!(pfa.alloc(), Some(0x8020_1000));
        assert_eq!(pfa.alloc(), Some(0x8020_2000));
        assert_eq!(pfa.alloc(), None);
    }

    #[test]
    fn syscall_decode_maps_write_and_exit() {
        assert_eq!(
            decode_syscall(SYS_WRITE, 1, 0x1000, 5),
            Some(Syscall::Write {
                fd: 1,
                ptr: 0x1000,
                len: 5
            })
        );
        assert_eq!(decode_syscall(SYS_EXIT, 7, 0, 0), Some(Syscall::Exit { code: 7 }));
    }

    #[test]
    fn syscall_handler_returns_exit() {
        assert_eq!(handle_syscall(SYS_EXIT, 3, 0, 0), SyscallResult::Exit(3));
    }

    #[test]
    fn trap_handler_advances_sepc_on_user_ecall() {
        let mut frame = TrapFrame::default();
        frame.sepc = 0x8020_1000;
        frame.set_a7(SYS_WRITE);
        frame.set_a0(1);
        frame.set_a1(0x9000_0000);
        frame.set_a2(4);

        assert_eq!(
            handle_trap(&mut frame, SCAUSE_USER_ECALL, 0),
            TrapOutcome::Resume
        );
        assert_eq!(frame.sepc, 0x8020_1004);
    }

    #[test]
    fn sv39_leaf_entry_keeps_ppn_and_flags() {
        let pte = PageTableEntry::leaf(0x80200, PTE_R | PTE_W | PTE_X | PTE_A | PTE_D);
        assert_eq!(pte.ppn(), 0x80200);
        assert_eq!(pte.flags() & (PTE_R | PTE_W | PTE_X), PTE_R | PTE_W | PTE_X);
    }

    #[test]
    fn sv39_satp_has_mode_8() {
        let satp = make_satp(0x12345, 0);
        assert_eq!(satp >> 60, 8);
    }

    #[test]
    fn sv39_root_table_maps_identity_entry() {
        let mut root = RootPageTable::new();
        root.map_gib_identity(2, 0x20000, PTE_R | PTE_X);
        assert_eq!(root.entries[2].ppn(), 0x20000);
    }

    #[test]
    fn user_init_context_clears_spp_for_umode() {
        let ctx = build_user_init_context(0x8040_0000, 0x8050_0000);
        assert_eq!(ctx.sepc, 0x8040_0000);
        assert_eq!(ctx.sstatus & SSTATUS_SPP, 0);
    }

    #[test]
    fn user_init_demo_exits_with_zero() {
        assert_eq!(run_user_init_demo(), TrapOutcome::Exit(0));
    }

    fn build_test_ext4_image() -> Vec<u8> {
        let mut image = vec![0u8; 16 * 1024];
        write_superblock(&mut image);
        write_group_descriptor(&mut image);
        write_inode(&mut image, 2, 0x4000, 1024, 10);
        write_inode(&mut image, 12, 0x4000, 1024, 11);
        write_inode(&mut image, 13, 0x8000, 8, 12);
        write_root_dir(&mut image);
        write_sbin_dir(&mut image);
        image[12 * 1024..12 * 1024 + 8].copy_from_slice(b"\x7fELFinit");
        image
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
}
