// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::syscall;
#[cfg(not(target_arch = "riscv64"))]
use crate::{
    page_fault::{self, PageFaultResult},
    pmm::PageFrameManager,
    scheduler::Scheduler,
    vm::VmSpace,
};

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub struct TrapFrame {
    pub regs: [usize; 32],
    pub sepc: usize,
    pub sstatus: usize,
}

impl TrapFrame {
    pub fn sp(&self) -> usize {
        self.regs[2]
    }

    pub fn a0(&self) -> usize {
        self.regs[10]
    }

    pub fn a1(&self) -> usize {
        self.regs[11]
    }

    pub fn a2(&self) -> usize {
        self.regs[12]
    }

    pub fn a7(&self) -> usize {
        self.regs[17]
    }

    pub fn set_a0(&mut self, value: usize) {
        self.regs[10] = value;
    }

    pub fn set_a1(&mut self, value: usize) {
        self.regs[11] = value;
    }

    pub fn set_a2(&mut self, value: usize) {
        self.regs[12] = value;
    }

    pub fn set_a7(&mut self, value: usize) {
        self.regs[17] = value;
    }

    pub fn set_sp(&mut self, value: usize) {
        self.regs[2] = value;
    }
}

pub const SCAUSE_USER_ECALL: usize = 8;
pub const SCAUSE_SUPERVISOR_TIMER_INTERRUPT: usize = 0x8000_0000_0000_0005;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapOutcome {
    Resume,
    Exit(i32),
    Fatal(usize),
}

extern "C" fn trap_entry() {
    // practice placeholder: real asm entry will save registers to TrapFrame
}

pub fn install_trap_vector() {
    let handler = trap_entry as *const () as usize;
    write_stvec(handler);
}

pub fn enable_supervisor_interrupts() {
    set_interrupt_enable_bits();
}

#[cfg(target_arch = "riscv64")]
fn write_stvec(handler: usize) {
    unsafe {
        core::arch::asm!("csrw stvec, {}", in(reg) handler);
    }
}

#[cfg(not(target_arch = "riscv64"))]
fn write_stvec(_handler: usize) {}

#[cfg(target_arch = "riscv64")]
fn set_interrupt_enable_bits() {
    // SEIE(9) | STIE(5) | SSIE(1)
    let sie_mask: usize = (1 << 9) | (1 << 5) | (1 << 1);

    unsafe {
        core::arch::asm!("csrsi sstatus, 0x2");
        core::arch::asm!("csrs sie, {}", in(reg) sie_mask);
    }
}

#[cfg(not(target_arch = "riscv64"))]
fn set_interrupt_enable_bits() {}

pub fn handle_trap(frame: &mut TrapFrame, scause: usize, _stval: usize) -> TrapOutcome {
    match scause {
        SCAUSE_USER_ECALL => match syscall::handle_syscall(frame.a7(), frame.a0(), frame.a1(), frame.a2()) {
            syscall::SyscallResult::Continue(ret) => {
                frame.set_a0(ret as usize);
                frame.sepc = frame.sepc.wrapping_add(4);
                TrapOutcome::Resume
            }
            syscall::SyscallResult::Exit(code) => TrapOutcome::Exit(code),
            syscall::SyscallResult::Unsupported => {
                frame.set_a0(usize::MAX);
                frame.sepc = frame.sepc.wrapping_add(4);
                TrapOutcome::Resume
            }
        },
        SCAUSE_SUPERVISOR_TIMER_INTERRUPT => TrapOutcome::Resume,
        cause => TrapOutcome::Fatal(cause),
    }
}

#[cfg(not(target_arch = "riscv64"))]
pub fn handle_user_page_fault_outcome<const A: usize, const F: usize>(
    scause: usize,
    stval: usize,
    vm: &VmSpace<A>,
    pmm: &mut PageFrameManager<F>,
) -> TrapOutcome {
    match page_fault::handle_user_page_fault(scause, stval, vm, pmm) {
        PageFaultResult::Mapped(_) => TrapOutcome::Resume,
        PageFaultResult::InvalidAccess | PageFaultResult::OutOfMemory => TrapOutcome::Exit(-1),
        PageFaultResult::Ignored => TrapOutcome::Fatal(scause),
    }
}

#[cfg(not(target_arch = "riscv64"))]
pub fn handle_timer_tick_with_scheduler<const N: usize>(
    _frame: &mut TrapFrame,
    scheduler: &mut Scheduler<N>,
) -> TrapOutcome {
    let _ = scheduler.on_tick();
    TrapOutcome::Resume
}
