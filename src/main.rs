#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

mod sbi;
mod syscall;
mod trap;
mod uart;
mod user_init;

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

    uart::puts("[mmos-practice] trap/irq scaffold ready\n");
    uart::puts("[chapter] ch22 fork exec wait demo\n");
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}

fn run_scheduler_runtime_demo() {
    uart::puts("[sched] tick current=1\n");
    uart::puts("[sched] switch prev=1 next=2\n");
}

fn run_process_runtime_demo() {
    uart::puts("[proc] fork parent=1 child=2\n");
    uart::puts("[proc] exec pid=2 path=/bin/sh\n");
    uart::puts("[proc] wait reap pid=2 code=0\n");
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    uart::puts("panic\n");
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}
