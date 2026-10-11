// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::trap::TrapFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchRecord {
    pub prev_tid: usize,
    pub next_tid: usize,
}

pub fn save_context(dst: &mut TrapFrame, src: &TrapFrame) {
    *dst = *src;
}

pub fn restore_context(dst: &mut TrapFrame, src: &TrapFrame) {
    *dst = *src;
}

pub fn switch_context(
    prev_tid: usize,
    next_tid: usize,
    live_frame: &mut TrapFrame,
    prev_slot: &mut TrapFrame,
    next_slot: &TrapFrame,
) -> SwitchRecord {
    save_context(prev_slot, live_frame);
    restore_context(live_frame, next_slot);
    SwitchRecord { prev_tid, next_tid }
}
