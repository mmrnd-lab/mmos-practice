// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

pub const SSTATUS_SUM: usize = 1 << 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserAccessWindow {
    pub previous_sstatus: usize,
    pub enabled_sstatus: usize,
    pub restored_sstatus: usize,
}

pub const fn enable_user_memory_access_status(sstatus: usize) -> usize {
    sstatus | SSTATUS_SUM
}

pub const fn restore_user_memory_access_status(previous_sstatus: usize) -> usize {
    previous_sstatus
}

pub const fn user_access_window_from_status(previous_sstatus: usize) -> UserAccessWindow {
    UserAccessWindow {
        previous_sstatus,
        enabled_sstatus: enable_user_memory_access_status(previous_sstatus),
        restored_sstatus: restore_user_memory_access_status(previous_sstatus),
    }
}

pub const fn user_memory_access_enabled(sstatus: usize) -> bool {
    (sstatus & SSTATUS_SUM) != 0
}

#[cfg(target_arch = "riscv64")]
struct UserAccessGuard {
    previous_sstatus: usize,
}

#[cfg(target_arch = "riscv64")]
impl UserAccessGuard {
    unsafe fn enable() -> Self {
        let previous_sstatus = read_sstatus();
        write_sstatus(enable_user_memory_access_status(previous_sstatus));
        Self { previous_sstatus }
    }
}

#[cfg(target_arch = "riscv64")]
impl Drop for UserAccessGuard {
    fn drop(&mut self) {
        unsafe {
            write_sstatus(self.previous_sstatus);
        }
    }
}

pub fn with_user_memory_access<R>(f: impl FnOnce() -> R) -> R {
    #[cfg(target_arch = "riscv64")]
    {
        let _guard = unsafe { UserAccessGuard::enable() };
        f()
    }

    #[cfg(not(target_arch = "riscv64"))]
    {
        f()
    }
}

#[cfg(target_arch = "riscv64")]
fn read_sstatus() -> usize {
    let value: usize;
    unsafe {
        core::arch::asm!("csrr {value}, sstatus", value = out(reg) value);
    }
    value
}

#[cfg(target_arch = "riscv64")]
unsafe fn write_sstatus(value: usize) {
    unsafe {
        core::arch::asm!("csrw sstatus, {value}", value = in(reg) value);
    }
}
