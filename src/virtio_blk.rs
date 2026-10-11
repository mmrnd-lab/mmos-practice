// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::block::{BlockDevice, BlockError};
use crate::device::{Device, DeviceKind};

const VIRTIO_MMIO_BASE: usize = 0x1000_1000;
const VIRTIO_MMIO_STRIDE: usize = 0x1000;
const VIRTIO_MMIO_SLOTS: usize = 8;

const REG_MAGIC: usize = 0x000;
const REG_VERSION: usize = 0x004;
const REG_DEVICE_ID: usize = 0x008;
const REG_DRIVER_FEATURES: usize = 0x020;
const REG_DRIVER_FEATURES_SEL: usize = 0x024;
const REG_QUEUE_SEL: usize = 0x030;
const REG_QUEUE_NUM_MAX: usize = 0x034;
const REG_QUEUE_NUM: usize = 0x038;
const REG_QUEUE_READY: usize = 0x044;
const REG_QUEUE_NOTIFY: usize = 0x050;
const REG_INTERRUPT_STATUS: usize = 0x060;
const REG_INTERRUPT_ACK: usize = 0x064;
const REG_STATUS: usize = 0x070;
const REG_QUEUE_DESC_LOW: usize = 0x080;
const REG_QUEUE_DESC_HIGH: usize = 0x084;
const REG_QUEUE_DRIVER_LOW: usize = 0x090;
const REG_QUEUE_DRIVER_HIGH: usize = 0x094;
const REG_QUEUE_DEVICE_LOW: usize = 0x0a0;
const REG_QUEUE_DEVICE_HIGH: usize = 0x0a4;

const VIRTIO_MAGIC: u32 = 0x7472_6976;
const VIRTIO_VERSION_MODERN: u32 = 2;
const VIRTIO_DEVICE_BLOCK: u32 = 2;

const STATUS_ACKNOWLEDGE: u32 = 1;
const STATUS_DRIVER: u32 = 2;
const STATUS_DRIVER_OK: u32 = 4;
const STATUS_FEATURES_OK: u32 = 8;
const STATUS_FAILED: u32 = 128;

const VIRTIO_F_VERSION_1: u32 = 1;
const QUEUE_SIZE: u16 = 8;
const DESC_F_NEXT: u16 = 1;
const DESC_F_WRITE: u16 = 2;
const VIRTIO_BLK_T_IN: u32 = 0;

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
struct VirtqAvail {
    flags: u16,
    idx: u16,
    ring: [u16; QUEUE_SIZE as usize],
    used_event: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtqUsedElem {
    id: u32,
    len: u32,
}

#[repr(C)]
struct VirtqUsed {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; QUEUE_SIZE as usize],
    avail_event: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtioBlkReqHeader {
    request_type: u32,
    reserved: u32,
    sector: u64,
}

#[repr(align(16))]
struct AlignedDesc([VirtqDesc; QUEUE_SIZE as usize]);

#[repr(align(2))]
struct AlignedAvail(VirtqAvail);

#[repr(align(4))]
struct AlignedUsed(VirtqUsed);

static mut DESCS: AlignedDesc = AlignedDesc(
    [VirtqDesc {
        addr: 0,
        len: 0,
        flags: 0,
        next: 0,
    }; QUEUE_SIZE as usize],
);
static mut AVAIL: AlignedAvail = AlignedAvail(VirtqAvail {
    flags: 0,
    idx: 0,
    ring: [0; QUEUE_SIZE as usize],
    used_event: 0,
});
static mut USED: AlignedUsed = AlignedUsed(VirtqUsed {
    flags: 0,
    idx: 0,
    ring: [VirtqUsedElem { id: 0, len: 0 }; QUEUE_SIZE as usize],
    avail_event: 0,
});
static mut HEADER: VirtioBlkReqHeader = VirtioBlkReqHeader {
    request_type: VIRTIO_BLK_T_IN,
    reserved: 0,
    sector: 0,
};
static mut STATUS_BYTE: u8 = 0xff;
static mut LAST_USED_IDX: u16 = 0;
static mut INITIALIZED_BASE: usize = 0;

#[derive(Debug, Clone, Copy)]
pub struct VirtioBlockDevice {
    base: usize,
}

impl VirtioBlockDevice {
    pub fn probe() -> Result<Self, BlockError> {
        let mut slot = 0usize;
        while slot < VIRTIO_MMIO_SLOTS {
            let base = VIRTIO_MMIO_BASE + slot * VIRTIO_MMIO_STRIDE;
            if read32(base, REG_MAGIC) == VIRTIO_MAGIC
                && read32(base, REG_VERSION) == VIRTIO_VERSION_MODERN
                && read32(base, REG_DEVICE_ID) == VIRTIO_DEVICE_BLOCK
            {
                let dev = Self { base };
                dev.init()?;
                return Ok(dev);
            }
            slot += 1;
        }
        Err(BlockError::Io)
    }

    fn init(&self) -> Result<(), BlockError> {
        unsafe {
            if INITIALIZED_BASE == self.base {
                return Ok(());
            }
        }

        write32(self.base, REG_STATUS, 0);
        self.add_status(STATUS_ACKNOWLEDGE);
        self.add_status(STATUS_DRIVER);

        write32(self.base, REG_DRIVER_FEATURES_SEL, 0);
        write32(self.base, REG_DRIVER_FEATURES, 0);
        write32(self.base, REG_DRIVER_FEATURES_SEL, 1);
        write32(self.base, REG_DRIVER_FEATURES, VIRTIO_F_VERSION_1);
        self.add_status(STATUS_FEATURES_OK);
        if read32(self.base, REG_STATUS) & STATUS_FEATURES_OK == 0 {
            self.fail();
            return Err(BlockError::Io);
        }

        write32(self.base, REG_QUEUE_SEL, 0);
        let max = read32(self.base, REG_QUEUE_NUM_MAX);
        if max < QUEUE_SIZE as u32 {
            self.fail();
            return Err(BlockError::Io);
        }

        write32(self.base, REG_QUEUE_NUM, QUEUE_SIZE as u32);

        unsafe {
            DESCS.0 = [VirtqDesc {
                addr: 0,
                len: 0,
                flags: 0,
                next: 0,
            }; QUEUE_SIZE as usize];
            AVAIL.0.flags = 0;
            AVAIL.0.idx = 0;
            AVAIL.0.ring = [0; QUEUE_SIZE as usize];
            USED.0.flags = 0;
            USED.0.idx = 0;
            USED.0.ring = [VirtqUsedElem { id: 0, len: 0 }; QUEUE_SIZE as usize];
            LAST_USED_IDX = 0;

            write_addr(
                self.base,
                REG_QUEUE_DESC_LOW,
                REG_QUEUE_DESC_HIGH,
                core::ptr::addr_of!(DESCS.0) as u64,
            );
            write_addr(
                self.base,
                REG_QUEUE_DRIVER_LOW,
                REG_QUEUE_DRIVER_HIGH,
                core::ptr::addr_of!(AVAIL.0) as u64,
            );
            write_addr(
                self.base,
                REG_QUEUE_DEVICE_LOW,
                REG_QUEUE_DEVICE_HIGH,
                core::ptr::addr_of!(USED.0) as u64,
            );
        }

        write32(self.base, REG_QUEUE_READY, 1);
        self.add_status(STATUS_DRIVER_OK);
        unsafe {
            INITIALIZED_BASE = self.base;
        }
        Ok(())
    }

    fn add_status(&self, bit: u32) {
        let status = read32(self.base, REG_STATUS);
        write32(self.base, REG_STATUS, status | bit);
    }

    fn fail(&self) {
        self.add_status(STATUS_FAILED);
    }
}

impl Device for VirtioBlockDevice {
    fn name(&self) -> &str {
        "virtio-blk0"
    }

    fn kind(&self) -> DeviceKind {
        DeviceKind::Block
    }
}

impl BlockDevice for VirtioBlockDevice {
    fn block_size(&self) -> usize {
        512
    }

    fn read_block(&self, lba: u64, out: &mut [u8]) -> Result<(), BlockError> {
        if out.len() != 512 {
            return Err(BlockError::BadBuffer);
        }

        unsafe {
            HEADER = VirtioBlkReqHeader {
                request_type: VIRTIO_BLK_T_IN,
                reserved: 0,
                sector: lba,
            };
            STATUS_BYTE = 0xff;

            DESCS.0[0] = VirtqDesc {
                addr: core::ptr::addr_of!(HEADER) as u64,
                len: core::mem::size_of::<VirtioBlkReqHeader>() as u32,
                flags: DESC_F_NEXT,
                next: 1,
            };
            DESCS.0[1] = VirtqDesc {
                addr: out.as_mut_ptr() as u64,
                len: out.len() as u32,
                flags: DESC_F_NEXT | DESC_F_WRITE,
                next: 2,
            };
            DESCS.0[2] = VirtqDesc {
                addr: core::ptr::addr_of!(STATUS_BYTE) as u64,
                len: 1,
                flags: DESC_F_WRITE,
                next: 0,
            };

            let avail_idx = AVAIL.0.idx;
            AVAIL.0.ring[(avail_idx as usize) % QUEUE_SIZE as usize] = 0;
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            AVAIL.0.idx = avail_idx.wrapping_add(1);
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
            write32(self.base, REG_QUEUE_NOTIFY, 0);

            while core::ptr::read_volatile(core::ptr::addr_of!(USED.0.idx)) == LAST_USED_IDX {
                core::hint::spin_loop();
            }
            LAST_USED_IDX = LAST_USED_IDX.wrapping_add(1);

            let interrupt = read32(self.base, REG_INTERRUPT_STATUS);
            if interrupt != 0 {
                write32(self.base, REG_INTERRUPT_ACK, interrupt);
            }

            if core::ptr::read_volatile(core::ptr::addr_of!(STATUS_BYTE)) == 0 {
                Ok(())
            } else {
                Err(BlockError::Io)
            }
        }
    }
}

fn read32(base: usize, reg: usize) -> u32 {
    unsafe { core::ptr::read_volatile((base + reg) as *const u32) }
}

fn write32(base: usize, reg: usize, value: u32) {
    unsafe { core::ptr::write_volatile((base + reg) as *mut u32, value) }
}

fn write_addr(base: usize, low_reg: usize, high_reg: usize, addr: u64) {
    write32(base, low_reg, addr as u32);
    write32(base, high_reg, (addr >> 32) as u32);
}
