// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::{scheduler::Scheduler, syscall};
#[cfg(not(target_arch = "riscv64"))]
use crate::{
    page_fault::{self, PageFaultResult},
    pmm::PageFrameManager,
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
pub const SCAUSE_SUPERVISOR_ECALL: usize = 9;
pub const SCAUSE_SUPERVISOR_TIMER_INTERRUPT: usize = 0x8000_0000_0000_0005;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapOutcome {
    Resume,
    Schedule,
    Exit(i32),
    Fatal(usize),
}

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    r#"
    .section .text.trap, "ax"
    .align 2
    .global __mmos_trap_entry
__mmos_trap_entry:
    csrw sscratch, sp
    addi sp, sp, -272

    sd zero,   0(sp)
    sd ra,     8(sp)
    sd zero,  16(sp)
    sd gp,    24(sp)
    sd tp,    32(sp)
    sd t0,    40(sp)
    sd t1,    48(sp)
    sd t2,    56(sp)
    sd s0,    64(sp)
    sd s1,    72(sp)
    sd a0,    80(sp)
    sd a1,    88(sp)
    sd a2,    96(sp)
    sd a3,   104(sp)
    sd a4,   112(sp)
    sd a5,   120(sp)
    sd a6,   128(sp)
    sd a7,   136(sp)
    sd s2,   144(sp)
    sd s3,   152(sp)
    sd s4,   160(sp)
    sd s5,   168(sp)
    sd s6,   176(sp)
    sd s7,   184(sp)
    sd s8,   192(sp)
    sd s9,   200(sp)
    sd s10,  208(sp)
    sd s11,  216(sp)
    sd t3,   224(sp)
    sd t4,   232(sp)
    sd t5,   240(sp)
    sd t6,   248(sp)
    csrr t6, sscratch
    sd t6,    16(sp)

    csrr t1, sepc
    sd t1,   256(sp)
    csrr t1, sstatus
    sd t1,   264(sp)

    mv a0, sp
    csrr a1, scause
    csrr a2, stval
    call rust_trap_dispatch

    ld t1,   256(sp)
    csrw sepc, t1
    ld t1,   264(sp)
    csrw sstatus, t1

    ld ra,     8(sp)
    ld gp,    24(sp)
    ld tp,    32(sp)
    ld t0,    40(sp)
    ld t1,    48(sp)
    ld t2,    56(sp)
    ld s0,    64(sp)
    ld s1,    72(sp)
    ld a0,    80(sp)
    ld a1,    88(sp)
    ld a2,    96(sp)
    ld a3,   104(sp)
    ld a4,   112(sp)
    ld a5,   120(sp)
    ld a6,   128(sp)
    ld a7,   136(sp)
    ld s2,   144(sp)
    ld s3,   152(sp)
    ld s4,   160(sp)
    ld s5,   168(sp)
    ld s6,   176(sp)
    ld s7,   184(sp)
    ld s8,   192(sp)
    ld s9,   200(sp)
    ld s10,  208(sp)
    ld s11,  216(sp)
    ld t3,   224(sp)
    ld t4,   232(sp)
    ld t5,   240(sp)
    ld t6,   248(sp)
    ld sp,    16(sp)
    sret
"#
);

#[cfg(target_arch = "riscv64")]
unsafe extern "C" {
    fn __mmos_trap_entry();
}

#[cfg(not(target_arch = "riscv64"))]
extern "C" fn trap_entry() {
    // practice placeholder: host tests call handle_trap directly.
}

pub fn install_trap_vector() {
    #[cfg(target_arch = "riscv64")]
    let handler = __mmos_trap_entry as usize;
    #[cfg(not(target_arch = "riscv64"))]
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
    // Keep interrupts masked until timer/context-switch path is fully wired.
}

#[cfg(not(target_arch = "riscv64"))]
fn set_interrupt_enable_bits() {}

pub fn handle_syscall_result(frame: &mut TrapFrame, result: syscall::SyscallResult) -> TrapOutcome {
    match result {
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
    }
}

pub fn handle_blocking_syscall_result(
    frame: &mut TrapFrame,
    result: syscall::BlockingReadResult,
) -> TrapOutcome {
    match result {
        syscall::BlockingReadResult::Ready(result) => handle_syscall_result(frame, result),
        syscall::BlockingReadResult::Blocked => TrapOutcome::Schedule,
    }
}

pub fn schedule_after_blocked_syscall<const N: usize>(
    frame: &TrapFrame,
    scheduler: &mut Scheduler<N>,
) -> Option<crate::scheduler::ScheduleDecision> {
    scheduler.schedule_after_current_blocked(*frame)
}

pub fn handle_trap(frame: &mut TrapFrame, scause: usize, _stval: usize) -> TrapOutcome {
    match scause {
        SCAUSE_USER_ECALL | SCAUSE_SUPERVISOR_ECALL => handle_syscall_result(
            frame,
            syscall::handle_syscall(frame.a7(), frame.a0(), frame.a1(), frame.a2()),
        ),
        SCAUSE_SUPERVISOR_TIMER_INTERRUPT => TrapOutcome::Resume,
        cause => TrapOutcome::Fatal(cause),
    }
}

#[cfg(target_arch = "riscv64")]
#[unsafe(no_mangle)]
extern "C" fn rust_trap_dispatch(frame: *mut TrapFrame, scause: usize, stval: usize) {
    let frame = unsafe { &mut *frame };
    match handle_trap(frame, scause, stval) {
        TrapOutcome::Resume | TrapOutcome::Schedule => {}
        TrapOutcome::Exit(code) => {
            crate::uart::puts("[user-init] exit=");
            put_i32(code);
            crate::uart::puts("\n");
            loop {
                unsafe { core::arch::asm!("wfi") }
            }
        }
        TrapOutcome::Fatal(cause) => {
            crate::uart::puts("[trap] fatal scause=0x");
            put_hex(cause);
            crate::uart::puts(" stval=0x");
            put_hex(stval);
            crate::uart::puts("\n");
            loop {
                unsafe { core::arch::asm!("wfi") }
            }
        }
    }
}

#[cfg(target_arch = "riscv64")]
fn put_i32(value: i32) {
    if value == 0 {
        crate::uart::puts("0");
        return;
    }

    let mut n = value as i64;
    if n < 0 {
        crate::uart::puts("-");
        n = -n;
    }

    let mut buf = [0u8; 20];
    let mut pos = buf.len();
    while n > 0 {
        pos -= 1;
        buf[pos] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    crate::uart::put_bytes(&buf[pos..]);
}

#[cfg(target_arch = "riscv64")]
fn put_hex(value: usize) {
    let mut started = false;
    for i in (0..(core::mem::size_of::<usize>() * 2)).rev() {
        let nib = ((value >> (i * 4)) & 0xf) as u8;
        if nib != 0 || started || i == 0 {
            started = true;
            let ch = if nib < 10 {
                b'0' + nib
            } else {
                b'a' + (nib - 10)
            };
            crate::uart::put_bytes(&[ch]);
        }
    }
}

#[cfg(target_arch = "riscv64")]
pub unsafe fn enter_user(frame: &TrapFrame) -> ! {
    let sepc = frame.sepc;
    let sstatus = frame.sstatus;
    let sp = frame.regs[2];
    let a0 = frame.regs[10];
    let a1 = frame.regs[11];
    let a2 = frame.regs[12];
    let a7 = frame.regs[17];

    core::arch::asm!(
        "csrw sepc, {sepc}",
        "csrw sstatus, {sstatus}",
        "mv sp, {usp}",
        "mv a0, {ua0}",
        "mv a1, {ua1}",
        "mv a2, {ua2}",
        "mv a7, {ua7}",
        "sret",
        sepc = in(reg) sepc,
        sstatus = in(reg) sstatus,
        usp = in(reg) sp,
        ua0 = in(reg) a0,
        ua1 = in(reg) a1,
        ua2 = in(reg) a2,
        ua7 = in(reg) a7,
        options(noreturn)
    );
}

#[cfg(not(target_arch = "riscv64"))]
pub fn enter_user(_frame: &TrapFrame) -> ! {
    panic!("enter_user is only available on riscv64 target")
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
