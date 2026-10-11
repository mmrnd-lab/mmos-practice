// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::scheduler::ScheduleDecision;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleAction {
    WaitForInterrupt { blocked_tid: usize },
}

pub fn idle_action_for_schedule_decision(decision: ScheduleDecision) -> Option<IdleAction> {
    match decision {
        ScheduleDecision::Switched { .. } => None,
        ScheduleDecision::Idle { blocked_tid } => {
            Some(IdleAction::WaitForInterrupt { blocked_tid })
        }
    }
}

#[cfg(target_arch = "riscv64")]
pub fn wait_for_interrupt_once() {
    unsafe { core::arch::asm!("wfi") }
}

#[cfg(not(target_arch = "riscv64"))]
pub fn wait_for_interrupt_once() {}
