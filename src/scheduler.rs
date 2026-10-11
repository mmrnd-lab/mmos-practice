// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::trap::TrapFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Runnable,
    Running,
    Blocked,
    Zombie,
}

#[derive(Debug, Clone, Copy)]
pub struct TaskSlot {
    pub tid: usize,
    pub state: TaskState,
    pub tf: TrapFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedError {
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleDecision {
    Switched { prev_tid: usize, next_tid: usize },
    Idle { blocked_tid: usize },
}

#[derive(Debug, Clone, Copy)]
pub struct Scheduler<const MAX_TASKS: usize> {
    slots: [Option<TaskSlot>; MAX_TASKS],
    current_idx: Option<usize>,
    tick_count: u64,
}

impl<const MAX_TASKS: usize> Scheduler<MAX_TASKS> {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_TASKS],
            current_idx: None,
            tick_count: 0,
        }
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    pub fn current_tid(&self) -> Option<usize> {
        self.current_idx
            .and_then(|idx| self.slots[idx].map(|task| task.tid))
    }

    pub fn task_state(&self, tid: usize) -> Option<TaskState> {
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if let Some(task) = self.slots[idx] {
                if task.tid == tid {
                    return Some(task.state);
                }
            }
            idx += 1;
        }
        None
    }

    pub fn task_frame(&self, tid: usize) -> Option<TrapFrame> {
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if let Some(task) = self.slots[idx] {
                if task.tid == tid {
                    return Some(task.tf);
                }
            }
            idx += 1;
        }
        None
    }

    pub fn add_runnable(&mut self, tid: usize, tf: TrapFrame) -> Result<(), SchedError> {
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if self.slots[idx].is_none() {
                self.slots[idx] = Some(TaskSlot {
                    tid,
                    state: TaskState::Runnable,
                    tf,
                });
                return Ok(());
            }
            idx += 1;
        }
        Err(SchedError::Full)
    }

    pub fn set_state(&mut self, tid: usize, state: TaskState) -> bool {
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if let Some(mut task) = self.slots[idx] {
                if task.tid == tid {
                    task.state = state;
                    self.slots[idx] = Some(task);
                    if state == TaskState::Zombie && self.current_idx == Some(idx) {
                        self.current_idx = None;
                    }
                    return true;
                }
            }
            idx += 1;
        }
        false
    }

    pub fn remove_task(&mut self, tid: usize) -> bool {
        let mut idx = 0usize;
        while idx < MAX_TASKS {
            if let Some(task) = self.slots[idx] {
                if task.tid == tid {
                    self.slots[idx] = None;
                    if self.current_idx == Some(idx) {
                        self.current_idx = None;
                    }
                    return true;
                }
            }
            idx += 1;
        }
        false
    }

    pub fn schedule_after_current_blocked(
        &mut self,
        current_tf: TrapFrame,
    ) -> Option<ScheduleDecision> {
        let cur = self.current_idx?;
        let mut current = self.slots[cur]?;
        let prev_tid = current.tid;
        current.tf = current_tf;
        self.slots[cur] = Some(current);

        let Some(next) = self.find_next_runnable((cur + 1) % MAX_TASKS) else {
            self.current_idx = None;
            return Some(ScheduleDecision::Idle {
                blocked_tid: prev_tid,
            });
        };

        if next == cur {
            self.current_idx = None;
            return Some(ScheduleDecision::Idle {
                blocked_tid: prev_tid,
            });
        }

        if let Some(mut task) = self.slots[next] {
            task.state = TaskState::Running;
            self.slots[next] = Some(task);
        }
        self.current_idx = Some(next);
        let next_tid = self.slots[next].map(|task| task.tid)?;
        Some(ScheduleDecision::Switched { prev_tid, next_tid })
    }

    pub fn schedule_after_current_exited(&mut self) -> Option<ScheduleDecision> {
        let cur = self.current_idx?;
        let mut current = self.slots[cur]?;
        let prev_tid = current.tid;
        current.state = TaskState::Zombie;
        self.slots[cur] = Some(current);

        let Some(next) = self.find_next_runnable((cur + 1) % MAX_TASKS) else {
            self.current_idx = None;
            return Some(ScheduleDecision::Idle {
                blocked_tid: prev_tid,
            });
        };

        if next == cur {
            self.current_idx = None;
            return Some(ScheduleDecision::Idle {
                blocked_tid: prev_tid,
            });
        }

        if let Some(mut task) = self.slots[next] {
            task.state = TaskState::Running;
            self.slots[next] = Some(task);
        }
        self.current_idx = Some(next);
        let next_tid = self.slots[next].map(|task| task.tid)?;
        Some(ScheduleDecision::Switched { prev_tid, next_tid })
    }

    pub fn on_tick(&mut self) -> Option<(usize, usize)> {
        self.tick_count = self.tick_count.wrapping_add(1);

        if self.current_idx.is_none() {
            let idx = self.find_next_runnable(0)?;
            let mut task = self.slots[idx]?;
            task.state = TaskState::Running;
            self.slots[idx] = Some(task);
            self.current_idx = Some(idx);
            return None;
        }

        let cur = self.current_idx.expect("checked above");
        if let Some(mut task) = self.slots[cur] {
            if task.state == TaskState::Running {
                task.state = TaskState::Runnable;
                self.slots[cur] = Some(task);
            }
        }

        let next = self.find_next_runnable((cur + 1) % MAX_TASKS).unwrap_or(cur);
        let prev_tid = self.slots[cur].map(|task| task.tid)?;
        let next_tid = self.slots[next].map(|task| task.tid)?;

        if next == cur {
            if let Some(mut task) = self.slots[cur] {
                task.state = TaskState::Running;
                self.slots[cur] = Some(task);
            }
            return None;
        }

        if let Some(mut task) = self.slots[next] {
            task.state = TaskState::Running;
            self.slots[next] = Some(task);
        }
        self.current_idx = Some(next);
        Some((prev_tid, next_tid))
    }

    fn find_next_runnable(&self, start: usize) -> Option<usize> {
        if MAX_TASKS == 0 {
            return None;
        }
        let mut scanned = 0usize;
        let mut idx = start % MAX_TASKS;
        while scanned < MAX_TASKS {
            if let Some(task) = self.slots[idx] {
                if task.state == TaskState::Runnable || task.state == TaskState::Running {
                    return Some(idx);
                }
            }
            idx = (idx + 1) % MAX_TASKS;
            scanned += 1;
        }
        None
    }
}
