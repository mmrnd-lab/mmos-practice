#![cfg_attr(not(test), no_std)]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0


pub mod block;
pub mod ramdisk;
pub mod block_io;
pub mod device;
pub mod irq_guard;
pub mod kheap;
pub mod page_frame_allocator;
pub mod syscall;
pub mod sv39;
pub mod trap;
pub mod user_init;

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
    use super::kheap::init_kheap;
    use super::page_frame_allocator::PageFrameAllocator;
    use super::syscall::{decode_syscall, handle_syscall, Syscall, SyscallResult, SYS_EXIT, SYS_WRITE};
    use super::sv39::{make_satp, PageTableEntry, RootPageTable, PTE_A, PTE_D, PTE_R, PTE_W, PTE_X};
    use super::trap::{handle_trap, TrapFrame, TrapOutcome, SCAUSE_USER_ECALL};
    use super::user_init::{build_user_init_context, run_user_init_demo, SSTATUS_SPP};

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
}
