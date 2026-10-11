// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::{
    context_switch,
    idle::{idle_action_for_schedule_decision, IdleAction},
    scheduler::{ScheduleDecision, Scheduler},
    syscall,
};
#[cfg(not(target_arch = "riscv64"))]
use crate::{
    page_fault::{self, PageFaultResult},
    pmm::PageFrameManager,
    vm::VmSpace,
};

pub const TRAP_FRAME_SIZE: usize = 272;
pub const KERNEL_TRAP_STACK_SIZE: usize = 4096;

#[repr(align(16))]
struct KernelTrapStack([u8; KERNEL_TRAP_STACK_SIZE]);

static KERNEL_TRAP_STACK: KernelTrapStack = KernelTrapStack([0; KERNEL_TRAP_STACK_SIZE]);

pub fn kernel_trap_stack_bottom() -> usize {
    core::ptr::addr_of!(KERNEL_TRAP_STACK.0) as usize
}

pub fn kernel_trap_stack_top() -> usize {
    kernel_trap_stack_bottom() + KERNEL_TRAP_STACK_SIZE
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub struct TrapFrame {
    pub regs: [usize; 32],
    pub sepc: usize,
    pub sstatus: usize,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelTrapStackSwitch {
    pub user_sp: usize,
    pub kernel_frame_sp: usize,
    pub frame_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrapEntryScratch {
    pub sscratch: usize,
    pub user_sp: usize,
    pub kernel_frame_sp: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrapEntryStackSwap {
    pub active_sp: usize,
    pub saved_user_sp: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrapEntryReturnScratch {
    pub restored_user_sp: usize,
    pub next_sscratch: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelTrapStackError {
    StackTooSmall,
    MisalignedTop,
}

pub fn prepare_kernel_trap_stack_switch(
    user_sp: usize,
    kernel_stack_top: usize,
    kernel_stack_bottom: usize,
) -> Result<KernelTrapStackSwitch, KernelTrapStackError> {
    if kernel_stack_top & 0xf != 0 {
        return Err(KernelTrapStackError::MisalignedTop);
    }
    let Some(kernel_frame_sp) = kernel_stack_top.checked_sub(TRAP_FRAME_SIZE) else {
        return Err(KernelTrapStackError::StackTooSmall);
    };
    if kernel_frame_sp < kernel_stack_bottom {
        return Err(KernelTrapStackError::StackTooSmall);
    }

    Ok(KernelTrapStackSwitch {
        user_sp,
        kernel_frame_sp,
        frame_size: TRAP_FRAME_SIZE,
    })
}

pub fn prepare_trap_entry_scratch(
    user_sp: usize,
    kernel_stack_top: usize,
    kernel_stack_bottom: usize,
) -> Result<TrapEntryScratch, KernelTrapStackError> {
    let switch =
        prepare_kernel_trap_stack_switch(user_sp, kernel_stack_top, kernel_stack_bottom)?;

    Ok(TrapEntryScratch {
        sscratch: switch.kernel_frame_sp,
        user_sp: switch.user_sp,
        kernel_frame_sp: switch.kernel_frame_sp,
    })
}

pub fn simulate_trap_entry_stack_swap(scratch: TrapEntryScratch) -> TrapEntryStackSwap {
    TrapEntryStackSwap {
        active_sp: scratch.sscratch,
        saved_user_sp: scratch.user_sp,
    }
}

pub fn simulate_trap_entry_return_scratch(scratch: TrapEntryScratch) -> TrapEntryReturnScratch {
    TrapEntryReturnScratch {
        restored_user_sp: scratch.user_sp,
        next_sscratch: scratch.kernel_frame_sp,
    }
}

pub fn prepare_enter_user_scratch(
    frame: &TrapFrame,
) -> Result<TrapEntryScratch, KernelTrapStackError> {
    prepare_trap_entry_scratch(frame.sp(), kernel_trap_stack_top(), kernel_trap_stack_bottom())
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

#[cfg(target_arch = "riscv64")]
static mut FIRST_USER_SYSCALL_TRAP_REPORTED: bool = false;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapOutcome {
    Resume,
    Schedule,
    Exit(i32),
    Fatal(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapDispatchResult {
    Resumed,
    Switched(ScheduleDecision),
    Idle(IdleAction),
    Exit(i32),
    Fatal(usize),
    MissingScheduleState,
}

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    r#"
    .section .text.trap, "ax"
    .align 2
    .global __mmos_trap_entry
__mmos_trap_entry:
    csrrw sp, sscratch, sp

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
    csrw sscratch, sp
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

#[allow(function_casts_as_integer)]
pub fn install_trap_vector() {
    #[cfg(target_arch = "riscv64")]
    let handler = __mmos_trap_entry as usize;
    #[cfg(not(target_arch = "riscv64"))]
    let handler = trap_entry as *const () as usize;
    write_stvec(handler);
}

#[allow(function_casts_as_integer)]
pub fn trap_vector_address() -> usize {
    #[cfg(target_arch = "riscv64")]
    let handler = __mmos_trap_entry as usize;
    #[cfg(not(target_arch = "riscv64"))]
    let handler = trap_entry as *const () as usize;
    handler
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

pub fn handle_current_execv_frame_from_runtime<
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
>(
    frame: &mut TrapFrame,
    ctx: syscall::CurrentExecvRuntimeContext<
        '_,
        '_,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >,
) -> TrapOutcome {
    let result = syscall::dispatch_current_execv_frame_from_runtime::<
        MAX_ARGS,
        ARG_BYTES,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >(frame, ctx);
    handle_syscall_result(frame, result)
}

pub struct ExecvTrapRuntimeResources<
    'a,
    'v,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
> {
    pub runtime: &'a mut crate::trap_runtime::ProcessTrapRuntime<
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >,
    pub user_bytes: &'a [u8],
    pub user_base: usize,
    pub vfs: &'a crate::vfs::Vfs<'v>,
    pub scratch: &'a mut [u8],
    pub stack_bytes: &'a mut [u8],
    pub stack_top_va: usize,
    pub new_vm: crate::vm::VmSpace<MAX_AREAS>,
}

pub struct ExecvTrapResourceStore<
    'v,
    const SCRATCH_BYTES: usize,
    const STACK_BYTES: usize,
    const MAX_AREAS: usize,
> {
    pub vfs: &'v crate::vfs::Vfs<'v>,
    pub scratch: [u8; SCRATCH_BYTES],
    pub stack_bytes: [u8; STACK_BYTES],
    pub stack_top_va: usize,
}

impl<'v, const SCRATCH_BYTES: usize, const STACK_BYTES: usize, const MAX_AREAS: usize>
    ExecvTrapResourceStore<'v, SCRATCH_BYTES, STACK_BYTES, MAX_AREAS>
{
    pub const fn new(vfs: &'v crate::vfs::Vfs<'v>, stack_top_va: usize) -> Self {
        Self {
            vfs,
            scratch: [0; SCRATCH_BYTES],
            stack_bytes: [0; STACK_BYTES],
            stack_top_va,
        }
    }

    pub fn resources<
        'a,
        const MAX_PROCS: usize,
        const MAX_CHILDREN: usize,
        const MAX_TASKS: usize,
    >(
        &'a mut self,
        runtime: &'a mut crate::trap_runtime::ProcessTrapRuntime<
            MAX_PROCS,
            MAX_CHILDREN,
            MAX_AREAS,
            MAX_TASKS,
        >,
        user_bytes: &'a [u8],
        user_base: usize,
        new_vm: crate::vm::VmSpace<MAX_AREAS>,
    ) -> ExecvTrapRuntimeResources<'a, 'v, MAX_PROCS, MAX_CHILDREN, MAX_AREAS, MAX_TASKS> {
        ExecvTrapRuntimeResources {
            runtime,
            user_bytes,
            user_base,
            vfs: self.vfs,
            scratch: &mut self.scratch,
            stack_bytes: &mut self.stack_bytes,
            stack_top_va: self.stack_top_va,
            new_vm,
        }
    }
}

impl<
        'a,
        'v,
        const MAX_PROCS: usize,
        const MAX_CHILDREN: usize,
        const MAX_AREAS: usize,
        const MAX_TASKS: usize,
    > ExecvTrapRuntimeResources<'a, 'v, MAX_PROCS, MAX_CHILDREN, MAX_AREAS, MAX_TASKS>
{
    pub fn into_current_context(
        self,
    ) -> syscall::CurrentExecvRuntimeContext<
        'a,
        'v,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    > {
        syscall::CurrentExecvRuntimeContext {
            runtime: self.runtime,
            user_bytes: self.user_bytes,
            user_base: self.user_base,
            vfs: self.vfs,
            scratch: self.scratch,
            stack_bytes: self.stack_bytes,
            stack_top_va: self.stack_top_va,
            new_vm: self.new_vm,
        }
    }
}

pub fn handle_trap_with_execv_resources<
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
>(
    frame: &mut TrapFrame,
    scause: usize,
    stval: usize,
    resources: ExecvTrapRuntimeResources<
        '_,
        '_,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >,
) -> TrapOutcome {
    handle_trap_with_current_execv_runtime::<
        MAX_ARGS,
        ARG_BYTES,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >(frame, scause, stval, resources.into_current_context())
}

pub fn handle_trap_with_current_execv_runtime<
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
>(
    frame: &mut TrapFrame,
    scause: usize,
    _stval: usize,
    ctx: syscall::CurrentExecvRuntimeContext<
        '_,
        '_,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
    >,
) -> TrapOutcome {
    match scause {
        SCAUSE_USER_ECALL | SCAUSE_SUPERVISOR_ECALL
            if frame.a7() == syscall::SYS_EXECVE =>
        {
            handle_current_execv_frame_from_runtime::<
                MAX_ARGS,
                ARG_BYTES,
                MAX_PROCS,
                MAX_CHILDREN,
                MAX_AREAS,
                MAX_TASKS,
            >(frame, ctx)
        }
        SCAUSE_USER_ECALL | SCAUSE_SUPERVISOR_ECALL => handle_syscall_result(
            frame,
            syscall::handle_syscall(frame.a7(), frame.a0(), frame.a1(), frame.a2()),
        ),
        SCAUSE_SUPERVISOR_TIMER_INTERRUPT => TrapOutcome::Resume,
        cause => TrapOutcome::Fatal(cause),
    }
}

pub fn handle_execv_user_ecall_with_prepared_runtime<
    'a,
    'v,
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
    const SCRATCH_BYTES: usize,
    const STACK_BYTES: usize,
>(
    frame: &mut TrapFrame,
    prepared: &'a mut crate::trap_runtime::PreparedExecvTrapRuntime<
        'v,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
        SCRATCH_BYTES,
        STACK_BYTES,
    >,
    user_bytes: &'a [u8],
    user_base: usize,
    new_vm: crate::vm::VmSpace<MAX_AREAS>,
) -> TrapOutcome {
    prepared.dispatch_execv_user_ecall::<MAX_ARGS, ARG_BYTES>(
        frame,
        user_bytes,
        user_base,
        new_vm,
    )
}

pub struct ExecvPreparedUserSource<'a> {
    pub bytes: &'a [u8],
    pub base: usize,
}

impl<'a> ExecvPreparedUserSource<'a> {
    pub const fn new(bytes: &'a [u8], base: usize) -> Self {
        Self { bytes, base }
    }
}

pub struct ExecvPreparedReplacementVm<const MAX_AREAS: usize> {
    pub vm: crate::vm::VmSpace<MAX_AREAS>,
}

impl<const MAX_AREAS: usize> ExecvPreparedReplacementVm<MAX_AREAS> {
    pub const fn new(vm: crate::vm::VmSpace<MAX_AREAS>) -> Self {
        Self { vm }
    }
}

pub struct ExecvPreparedEntryRecipe<'a, const MAX_AREAS: usize> {
    pub user_source: ExecvPreparedUserSource<'a>,
    pub replacement_vm: ExecvPreparedReplacementVm<MAX_AREAS>,
}

impl<'a, const MAX_AREAS: usize> ExecvPreparedEntryRecipe<'a, MAX_AREAS> {
    pub const fn new(
        user_source: ExecvPreparedUserSource<'a>,
        replacement_vm: ExecvPreparedReplacementVm<MAX_AREAS>,
    ) -> Self {
        Self {
            user_source,
            replacement_vm,
        }
    }

    pub const fn into_input(self) -> ExecvPreparedEntryInput<'a, MAX_AREAS> {
        ExecvPreparedEntryInput {
            user_bytes: self.user_source.bytes,
            user_base: self.user_source.base,
            new_vm: self.replacement_vm.vm,
        }
    }
}

pub struct ExecvPreparedEntryInput<'a, const MAX_AREAS: usize> {
    pub user_bytes: &'a [u8],
    pub user_base: usize,
    pub new_vm: crate::vm::VmSpace<MAX_AREAS>,
}

pub fn handle_execv_user_ecall_with_prepared_store<
    'a,
    'v,
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
    const SCRATCH_BYTES: usize,
    const STACK_BYTES: usize,
>(
    frame: &mut TrapFrame,
    store: &'a mut crate::trap_runtime::PreparedExecvEntryStore<
        'v,
        MAX_PROCS,
        MAX_CHILDREN,
        MAX_AREAS,
        MAX_TASKS,
        SCRATCH_BYTES,
        STACK_BYTES,
    >,
    input: ExecvPreparedEntryInput<'a, MAX_AREAS>,
) -> TrapOutcome {
    store
        .with_prepared_runtime(|prepared| {
            prepared.dispatch_execv_user_ecall::<MAX_ARGS, ARG_BYTES>(
                frame,
                input.user_bytes,
                input.user_base,
                input.new_vm,
            )
        })
        .unwrap_or(TrapOutcome::Fatal(usize::MAX))
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
) -> Option<ScheduleDecision> {
    scheduler.schedule_after_current_blocked(*frame)
}

pub fn apply_schedule_decision<const N: usize>(
    frame: &mut TrapFrame,
    scheduler: &Scheduler<N>,
    decision: ScheduleDecision,
) -> Option<ScheduleDecision> {
    match decision {
        ScheduleDecision::Switched { next_tid, .. } => {
            let next_frame = scheduler.task_frame(next_tid)?;
            context_switch::restore_context(frame, &next_frame);
            Some(decision)
        }
        ScheduleDecision::Idle { .. } => Some(decision),
    }
}

pub fn dispatch_trap_outcome<const N: usize>(
    frame: &mut TrapFrame,
    scheduler: &mut Scheduler<N>,
    outcome: TrapOutcome,
) -> TrapDispatchResult {
    match outcome {
        TrapOutcome::Resume => TrapDispatchResult::Resumed,
        TrapOutcome::Exit(code) => TrapDispatchResult::Exit(code),
        TrapOutcome::Fatal(cause) => TrapDispatchResult::Fatal(cause),
        TrapOutcome::Schedule => {
            let Some(decision) = schedule_after_blocked_syscall(frame, scheduler) else {
                return TrapDispatchResult::MissingScheduleState;
            };
            if let Some(action) = idle_action_for_schedule_decision(decision) {
                return TrapDispatchResult::Idle(action);
            }
            match apply_schedule_decision(frame, scheduler, decision) {
                Some(decision) => TrapDispatchResult::Switched(decision),
                None => TrapDispatchResult::MissingScheduleState,
            }
        }
    }
}

pub fn dispatch_trap_outcome_with_global_runtime(
    frame: &mut TrapFrame,
    outcome: TrapOutcome,
) -> TrapDispatchResult {
    match outcome {
        TrapOutcome::Schedule => {
            crate::trap_runtime::with_global_trap_runtime(|runtime| {
                runtime.dispatch_schedule(frame)
            })
        }
        TrapOutcome::Resume => TrapDispatchResult::Resumed,
        TrapOutcome::Exit(code) => TrapDispatchResult::Exit(code),
        TrapOutcome::Fatal(cause) => TrapDispatchResult::Fatal(cause),
    }
}

pub fn dispatch_exit_outcome_with_process_runtime<
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
    const SCHED_TASKS: usize,
>(
    frame: &mut TrapFrame,
    processes: &mut crate::process_model::ProcessTable<MAX_PROCS, MAX_CHILDREN, MAX_AREAS>,
    tasks: &mut crate::process_model::TaskTable<MAX_TASKS>,
    scheduler: &mut Scheduler<SCHED_TASKS>,
    current_tid: usize,
    code: i32,
) -> TrapDispatchResult {
    let Ok(report) = crate::process_model::exit_process_from_task_with_scheduler(
        processes,
        tasks,
        scheduler,
        current_tid,
        code,
    ) else {
        return TrapDispatchResult::MissingScheduleState;
    };

    if let Some(action) = idle_action_for_schedule_decision(report.decision) {
        return TrapDispatchResult::Idle(action);
    }

    match apply_schedule_decision(frame, scheduler, report.decision) {
        Some(decision) => TrapDispatchResult::Switched(decision),
        None => TrapDispatchResult::MissingScheduleState,
    }
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
    if scause == SCAUSE_USER_ECALL
        && !unsafe { core::ptr::read_volatile(core::ptr::addr_of!(FIRST_USER_SYSCALL_TRAP_REPORTED)) }
    {
        unsafe {
            core::ptr::write_volatile(
                core::ptr::addr_of_mut!(FIRST_USER_SYSCALL_TRAP_REPORTED),
                true,
            );
        }
        crate::console::puts("[chapter] ch87 first user syscall trap\n");
    }
    let outcome = handle_trap(frame, scause, stval);
    match dispatch_trap_outcome_with_global_runtime(frame, outcome) {
        TrapDispatchResult::Resumed | TrapDispatchResult::Switched(_) => {}
        TrapDispatchResult::Idle(_) => loop {
            crate::idle::wait_for_interrupt_once();
        },
        TrapDispatchResult::Exit(code) => {
            crate::console::puts("[user-init] exit=");
            put_i32(code);
            crate::console::puts("\n");
            loop {
                unsafe { core::arch::asm!("wfi") }
            }
        }
        TrapDispatchResult::Fatal(cause) => {
            crate::console::puts("[trap] fatal scause=0x");
            put_hex(cause);
            crate::console::puts(" stval=0x");
            put_hex(stval);
            crate::console::puts("\n");
            loop {
                unsafe { core::arch::asm!("wfi") }
            }
        }
        TrapDispatchResult::MissingScheduleState => {
            crate::console::puts("[trap] missing schedule state stval=0x");
            put_hex(stval);
            crate::console::puts("\n");
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
    let scratch = match prepare_enter_user_scratch(frame) {
        Ok(scratch) => scratch.sscratch,
        Err(_) => loop {
            core::arch::asm!("wfi")
        },
    };

    core::arch::asm!(
        "csrw sscratch, {scratch}",
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
        scratch = in(reg) scratch,
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
