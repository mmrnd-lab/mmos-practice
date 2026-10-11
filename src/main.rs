#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

mod sbi;
mod trap;
mod uart;

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
    uart::puts("\\n");
    uart::puts("[mmos-practice] trap/irq scaffold ready\\n");
    uart::puts("[chapter] ch08 trap scaffold\\n");
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    uart::puts("panic\\n");
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}
