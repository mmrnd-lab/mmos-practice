// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

pub struct IrqGuard;

impl IrqGuard {
    pub fn new() -> Self {
        unsafe {
            core::arch::asm!("csrci sstatus, 0x2");
        }
        Self
    }
}

impl Drop for IrqGuard {
    fn drop(&mut self) {
        unsafe {
            core::arch::asm!("csrsi sstatus, 0x2");
        }
    }
}
