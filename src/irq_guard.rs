// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

pub struct IrqGuard;

impl IrqGuard {
    pub fn new() -> Self {
        disable_irq_bit();
        Self
    }
}

impl Drop for IrqGuard {
    fn drop(&mut self) {
        enable_irq_bit();
    }
}

#[cfg(target_arch = "riscv64")]
fn disable_irq_bit() {
    unsafe {
        core::arch::asm!("csrci sstatus, 0x2");
    }
}

#[cfg(not(target_arch = "riscv64"))]
fn disable_irq_bit() {}

#[cfg(target_arch = "riscv64")]
fn enable_irq_bit() {
    unsafe {
        core::arch::asm!("csrsi sstatus, 0x2");
    }
}

#[cfg(not(target_arch = "riscv64"))]
fn enable_irq_bit() {}
