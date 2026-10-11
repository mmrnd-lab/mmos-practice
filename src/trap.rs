// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub struct TrapFrame {
    pub regs: [usize; 32],
    pub sepc: usize,
    pub sstatus: usize,
}

extern "C" fn trap_entry() {
    // practice placeholder: real asm entry will save registers to TrapFrame
}

pub fn install_trap_vector() {
    let handler = trap_entry as usize;
    unsafe {
        core::arch::asm!("csrw stvec, {}", in(reg) handler);
    }
}

pub fn enable_supervisor_interrupts() {
    // SEIE(9) | STIE(5) | SSIE(1)
    let sie_mask: usize = (1 << 9) | (1 << 5) | (1 << 1);

    unsafe {
        core::arch::asm!("csrsi sstatus, 0x2");
        core::arch::asm!("csrs sie, {}", in(reg) sie_mask);
    }
}
