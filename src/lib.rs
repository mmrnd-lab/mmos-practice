#![cfg_attr(not(test), no_std)]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0


pub mod irq_guard;
pub mod kheap;
pub mod page_frame_allocator;
pub mod trap;

pub fn align_up(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (value + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::align_up;
    use super::kheap::init_kheap;
    use super::page_frame_allocator::PageFrameAllocator;

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
    fn page_frame_allocator_allocates_sequential_frames() {
        let mut pfa = PageFrameAllocator::new(0x8020_0000, 0x8020_3000, 0x1000);
        assert_eq!(pfa.alloc(), Some(0x8020_0000));
        assert_eq!(pfa.alloc(), Some(0x8020_1000));
        assert_eq!(pfa.alloc(), Some(0x8020_2000));
        assert_eq!(pfa.alloc(), None);
    }
}
