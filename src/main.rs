#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

mod sbi;
mod syscall;
mod trap;
mod uart;
mod user_init;
#[allow(dead_code)]
mod block;
#[allow(dead_code)]
mod block_io;
#[allow(dead_code)]
mod ramdisk;
#[allow(dead_code)]
mod device;
#[allow(dead_code)]
mod elf_loader;
#[allow(dead_code)]
mod ext4;
#[allow(dead_code)]
mod page_fault;
#[allow(dead_code)]
mod pmm;
#[allow(dead_code)]
mod process_model;
mod process_runtime;
mod scheduler;
#[allow(dead_code)]
mod sv39;
#[allow(dead_code)]
mod user_exec;
mod user_launch;
mod user_stack;
#[allow(dead_code)]
mod user_vm;
#[allow(dead_code)]
mod vm;
#[allow(dead_code)]
mod vfs;

use core::panic::PanicInfo;

#[cfg(feature = "console-mmio")]
const CONSOLE_KIND: &str = "mmio-uart";
#[cfg(feature = "console-sbi")]
const CONSOLE_KIND: &str = "sbi-console";

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    trap::install_trap_vector();
    trap::enable_supervisor_interrupts();

    uart::puts("[mmos-practice] console backend=");
    uart::puts(CONSOLE_KIND);
    uart::puts("\n");

    let mut boot_tf = trap::TrapFrame::default();
    boot_tf.set_sp(0x8050_0000);
    let _ = boot_tf.sp();
    boot_tf.set_a7(syscall::SYS_WRITE);
    boot_tf.set_a0(1);
    boot_tf.set_a1(0x9000_0000);
    boot_tf.set_a2(8);
    let _ = trap::handle_trap(&mut boot_tf, trap::SCAUSE_USER_ECALL, 0);

    let init_ctx = user_init::build_user_init_context(0x8040_0000, 0x8050_0000);
    if init_ctx.sstatus & user_init::SSTATUS_SPP == 0 {
        uart::puts("[mmos-practice] user init context prepared (SPP=U)\n");
    }

    if let trap::TrapOutcome::Exit(code) = user_init::run_user_init_demo() {
        if code == 0 {
            uart::puts("[mmos-practice] user init demo exit=0\n");
        }
    }

    run_scheduler_runtime_demo();
    run_process_runtime_demo();
    run_user_launch_demo();

    uart::puts("[mmos-practice] trap/irq scaffold ready\n");
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}

fn run_scheduler_runtime_demo() {
    let mut sched = scheduler::Scheduler::<4>::new();
    let _ = sched.add_runnable(1, trap::TrapFrame::default());
    let _ = sched.add_runnable(2, trap::TrapFrame::default());
    let _ = sched.on_tick();
    if let Some((prev, next)) = sched.on_tick() {
        if prev == 1 && next == 2 {
            uart::puts("[sched] tick current=1\n");
            uart::puts("[sched] switch prev=1 next=2\n");
            uart::puts("[chapter] ch20 scheduler switch\n");
        }
    }
}

fn run_process_runtime_demo() {
    let demo = process_runtime::run_fork_exec_wait_demo();
    if let Ok(report) = demo {
        if report.parent_pid == 1 && report.child_pid == 2 && report.reap_code == 0 {
            uart::puts("[chapter] ch21 process model\n");
            uart::puts("[chapter] ch22 fork exec wait demo\n");
            uart::puts("[proc] fork parent=1 child=2\n");
            uart::puts("[proc] exec pid=2 path=/bin/sh\n");
            uart::puts("[proc] wait reap pid=2 code=0\n");
        }
    }
}

fn run_user_launch_demo() {
    let mut vm = vm::VmSpace::<4>::new();
    let _ = vm.map_anon(0x8040_0000, 0x8040_1000, sv39::PTE_R | sv39::PTE_W);
    let mut pmm = pmm::PageFrameManager::<4>::new(0x9000_0000, 0x9000_4000, sv39::PAGE_SIZE);
    let pf = page_fault::handle_user_page_fault(
        page_fault::SCAUSE_STORE_PAGE_FAULT,
        0x8040_0010,
        &vm,
        &mut pmm,
    );
    if let page_fault::PageFaultResult::Mapped(_) = pf {
        uart::puts("[chapter] ch19 trap outcome bridge\n");
    }

    let code = [0x13, 0x00, 0x00, 0x00];
    let loaded = elf_loader::LoadedUserImage {
        entry: 0x8041_0000,
        stack_top: 0x8050_0000,
        segments: [
            Some(elf_loader::LoadSegment {
                vaddr: 0x8041_0000,
                file_size: 4,
                mem_size: 8,
                flags: 5,
                data: &code,
            }),
            None,
            None,
            None,
        ],
        segment_count: 1,
    };
    let mut stack = [0u8; 256];
    let plan = user_launch::prepare_user_init_launch::<4, 4>(
        &loaded,
        0x12345,
        &mut stack,
        &["/sbin/init"],
        &["PATH=/bin"],
    );
    if let Ok(p) = plan {
        if p.argc == 1 && p.trapframe.sepc == 0x8041_0000 {
            uart::puts("[chapter] ch26 user init launch handoff\n");
            uart::puts("[launch] vm+stack+trapframe ready for first sret\n");
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    uart::puts("panic\n");
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}
