// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::syscall;
use crate::trap;

pub const SSTATUS_SPP: usize = 1 << 8;
pub const SSTATUS_SPIE: usize = 1 << 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserInitContext {
    pub sepc: usize,
    pub sstatus: usize,
    pub user_sp: usize,
}

pub fn build_user_init_context(entry: usize, user_sp: usize) -> UserInitContext {
    // SPP=0 keeps the next return target at U-mode.
    let sstatus = SSTATUS_SPIE;
    UserInitContext {
        sepc: entry,
        sstatus,
        user_sp,
    }
}

pub fn run_user_init_demo() -> trap::TrapOutcome {
    let mut frame = trap::TrapFrame::default();
    frame.sepc = 0x8040_0000;
    frame.set_a7(syscall::SYS_EXIT);
    frame.set_a0(0);

    trap::handle_trap(&mut frame, trap::SCAUSE_USER_ECALL, 0)
}
