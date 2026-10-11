// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "console-mmio")]
use core::fmt::{self, Write};
#[cfg(feature = "console-mmio")]
use core::sync::atomic::{AtomicBool, Ordering};

use crate::sbi;

#[cfg(feature = "console-mmio")]
pub const UART0_BASE: usize = 0x1000_0000;
#[cfg(feature = "console-mmio")]
static UART_READY: AtomicBool = AtomicBool::new(false);

#[cfg(all(feature = "console-sbi", feature = "console-mmio"))]
compile_error!("select only one console backend");

#[cfg(not(any(feature = "console-sbi", feature = "console-mmio")))]
compile_error!("select a console backend feature");

#[cfg(feature = "console-mmio")]
pub struct Uart {
    base: usize,
}

#[cfg(feature = "console-mmio")]
impl Uart {
    pub const unsafe fn from_base(base: usize) -> Self {
        Self { base }
    }

    #[inline(always)]
    unsafe fn write_byte(&mut self, c: u8) {
        let lsr = (self.base + 0x05) as *const u8;
        let thr = (self.base + 0x00) as *mut u8;

        while core::ptr::read_volatile(lsr) & 0x20 == 0 {}
        core::ptr::write_volatile(thr, c);
    }

    pub unsafe fn init(&mut self) {
        let lcr = (self.base + 0x03) as *mut u8;
        let ier = (self.base + 0x01) as *mut u8;
        let fcr = (self.base + 0x02) as *mut u8;
        let dll = (self.base + 0x00) as *mut u8;
        let dlh = (self.base + 0x01) as *mut u8;

        core::ptr::write_volatile(ier, 0x00);
        core::ptr::write_volatile(lcr, 0x80);
        core::ptr::write_volatile(dll, 1);
        core::ptr::write_volatile(dlh, 0);
        core::ptr::write_volatile(lcr, 0x03);
        core::ptr::write_volatile(fcr, 0x07);
    }
}

#[cfg(feature = "console-mmio")]
impl Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            unsafe { self.write_byte(byte) };
        }
        Ok(())
    }
}

#[cfg(feature = "console-sbi")]
pub fn put_byte_at(_base: usize, byte: u8) {
    sbi::console_putchar(byte);
}

#[cfg(feature = "console-sbi")]
pub fn puts(s: &str) {
    for b in s.bytes() {
        sbi::console_putchar(b);
    }
}

#[cfg(feature = "console-sbi")]
pub fn put_bytes_at(base: usize, bytes: &[u8]) {
    for &b in bytes {
        put_byte_at(base, b);
    }
}

#[cfg(feature = "console-sbi")]
pub fn put_bytes(bytes: &[u8]) {
    put_bytes_at(0, bytes);
}

#[cfg(feature = "console-mmio")]
pub fn put_byte_at(base: usize, byte: u8) {
    let mut uart = unsafe { Uart::from_base(base) };

    if !UART_READY.swap(true, Ordering::AcqRel) {
        unsafe { uart.init() };
    }

    unsafe { uart.write_byte(byte) };
}

#[cfg(feature = "console-mmio")]
pub fn puts(s: &str) {
    let mut uart = unsafe { Uart::from_base(UART0_BASE) };

    if !UART_READY.swap(true, Ordering::AcqRel) {
        unsafe { uart.init() };
    }

    let _ = uart.write_str(s);
}

#[cfg(feature = "console-mmio")]
pub fn put_bytes_at(base: usize, bytes: &[u8]) {
    let mut uart = unsafe { Uart::from_base(base) };

    if !UART_READY.swap(true, Ordering::AcqRel) {
        unsafe { uart.init() };
    }

    for &b in bytes {
        unsafe { uart.write_byte(b) };
    }
}

#[cfg(feature = "console-mmio")]
pub fn put_bytes(bytes: &[u8]) {
    put_bytes_at(UART0_BASE, bytes);
}
