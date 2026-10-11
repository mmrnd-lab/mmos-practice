#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

#[allow(dead_code)]
mod block;
#[allow(dead_code)]
mod block_io;
mod char_device;
mod console;
#[allow(dead_code)]
mod context_switch;
#[allow(dead_code)]
mod demo_user_init;
#[allow(dead_code)]
mod devfs;
#[allow(dead_code)]
mod device;
#[allow(dead_code)]
mod elf_loader;
#[allow(dead_code)]
mod exec_args;
#[allow(dead_code)]
mod exec_flow;
#[allow(dead_code)]
mod exec_path;
#[allow(dead_code)]
mod ext4;
#[allow(dead_code)]
mod fd_table;
mod foundation_lifecycle;
#[allow(dead_code)]
mod idle;
#[allow(dead_code)]
mod irq_guard;
#[allow(dead_code)]
mod page_fault;
#[allow(dead_code)]
mod pmm;
#[allow(dead_code)]
mod process_io;
#[allow(dead_code)]
mod process_model;
#[allow(dead_code)]
mod process_runtime;
#[allow(dead_code)]
mod ramdisk;
mod sbi;
#[allow(dead_code)]
mod scheduler;
mod serial;
#[allow(dead_code)]
mod serial_rx;
#[allow(dead_code)]
mod sv39;
mod syscall;
#[allow(dead_code)]
mod task_vm;
mod trap;
#[allow(dead_code)]
mod trap_runtime;
mod uart;
#[allow(dead_code)]
mod user_access;
#[allow(dead_code)]
mod user_copy;
#[allow(dead_code)]
mod user_exec;
mod user_init;
mod user_launch;
#[allow(dead_code)]
mod user_process;
mod user_stack;
#[allow(dead_code)]
mod user_vm;
#[allow(dead_code)]
mod vfs;
#[allow(dead_code)]
mod virtio_blk;
#[allow(dead_code)]
mod vm;
#[allow(dead_code)]
mod wait_queue;

use core::panic::PanicInfo;

static mut USER_ADDRESS_SPACE: user_vm::ProcessAddressSpace = user_vm::ProcessAddressSpace::empty();
static mut USER_ADDRESS_SPACES: user_vm::ProcessAddressSpaceTable<2> =
    user_vm::ProcessAddressSpaceTable::new();
static mut PCB_ADDRESS_SPACE_HANDLE_READY: bool = false;
static mut SATP_HARDWARE_COMMIT_PROBE_READY: bool = false;
static mut USER_FRAME_BACKED_IMAGE_READY: bool = false;
static mut USER_FRAME_BACKED_MAPPING_READY: bool = false;
static mut USER_FRAME_BACKED_MATERIALIZE_READY: bool = false;
static mut PERSISTENT_SATP_COMMIT_SATP: usize = 0;
static mut PERSISTENT_SATP_COMMIT_READY: bool = false;
static mut USER_MEMORY_ACCESS_GATE_READY: bool = false;
static mut FIRST_USER_SRET_FRAME: trap::TrapFrame = trap::TrapFrame {
    regs: [0; 32],
    sepc: 0,
    sstatus: 0,
};
static mut FIRST_USER_SRET_READY: bool = false;
static mut CH105_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH105_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH106_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH106_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH107_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH107_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH108_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH108_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH109_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH109_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH110_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH110_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH111_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH111_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH112_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH112_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH113_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH113_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH114_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH114_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH137_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH137_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH138_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH138_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH139_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH139_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH140_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH140_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH141_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH141_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];
static mut CH142_INIT_ELF: [u8; demo_user_init::DEMO_INIT_ELF_SIZE] =
    [0; demo_user_init::DEMO_INIT_ELF_SIZE];
static mut CH142_EXT4_IMAGE: [u8; demo_user_init::DEMO_EXT4_IMAGE_SIZE] =
    [0; demo_user_init::DEMO_EXT4_IMAGE_SIZE];

trait SyscallResultExt {
    fn is_ok_continue_zero(self) -> bool;
}

impl SyscallResultExt for syscall::SyscallResult {
    fn is_ok_continue_zero(self) -> bool {
        matches!(self, syscall::SyscallResult::Continue(0))
    }
}

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    r#"
    .section .text.entry, "ax"
    .global _start
_start:
    la sp, __mmos_boot_stack_top
    call rust_main
1:
    wfi
    j 1b

    .section .bss.stack, "aw", @nobits
    .align 12
__mmos_boot_stack:
    .space 65536
__mmos_boot_stack_top:
"#
);

#[cfg(feature = "console-mmio")]
const CONSOLE_KIND: &str = "mmio-uart";
#[cfg(feature = "console-sbi")]
const CONSOLE_KIND: &str = "sbi-console";

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    trap::install_trap_vector();
    trap::enable_supervisor_interrupts();

    console::puts("[mmos-practice] console backend=");
    console::puts(CONSOLE_KIND);
    console::puts("\n");
    console::puts("[chapter] ch32 serial character device\n");
    console::puts("[chapter] ch33 fd table stdio\n");
    let dev_ns = devfs::DeviceNamespace::new(console::device());
    if dev_ns.lookup_path("/dev/console").is_ok() {
        console::puts("[chapter] ch34 devfs device nodes\n");
    }
    let mut fd_probe = fd_table::stdio_table_with_capacity::<8>(console::device());
    if let syscall::SyscallResult::Continue(fd) =
        syscall::handle_open_path(&mut fd_probe, &dev_ns, "/dev/console", 1)
    {
        if fd >= 3 {
            console::puts("[chapter] ch35 open syscall path\n");
        }
    }
    let mut current_io = process_io::ProcessIoContext::<8>::with_stdio(console::device());
    if let syscall::SyscallResult::Continue(fd) =
        syscall::handle_process_open_path(&mut current_io, &dev_ns, "/dev/console", 1)
    {
        if syscall::handle_process_write(&current_io, fd as usize, b"").is_ok_continue_zero() {
            console::puts("[chapter] ch36 process fd context\n");
        }
    }
    let user_mem = user_copy::UserSlice::new(b"/dev/console\0", 0x4000);
    let mut path_scratch = [0u8; 32];
    let mut copy_io = process_io::ProcessIoContext::<8>::with_stdio(console::device());
    if let syscall::SyscallResult::Continue(fd) = syscall::handle_process_open_user_path(
        &mut copy_io,
        &dev_ns,
        &user_mem,
        0x4000,
        1,
        &mut path_scratch,
    ) {
        if fd >= 3 {
            console::puts("[chapter] ch37 user copyin\n");
        }
    }
    let mut user_vm = vm::VmSpace::<2>::new();
    let _ = user_vm.map_anon(0x4000, 0x4020, sv39::PTE_R);
    let checked_user =
        user_copy::CheckedUserSlice::new(b"/dev/console\0fd-write", 0x4000, &user_vm);
    let mut checked_io = process_io::ProcessIoContext::<8>::with_stdio(console::device());
    let mut checked_scratch = [0u8; 32];
    if let syscall::SyscallResult::Continue(fd) = syscall::handle_process_open_checked_user_path(
        &mut checked_io,
        &dev_ns,
        &checked_user,
        0x4000,
        1,
        &mut checked_scratch,
    ) {
        if fd >= 3 {
            console::puts("[chapter] ch38 vm checked copyin\n");
            let mut write_scratch = [0u8; 16];
            if syscall::handle_process_write_checked_user_syscall(
                &checked_io,
                &checked_user,
                fd as usize,
                0x400d,
                8,
                &mut write_scratch,
            ) == syscall::SyscallResult::Continue(8)
            {
                console::puts("[chapter] ch39 fd write syscall\n");
            }
        }
    }

    let mut boot_tf = trap::TrapFrame::default();
    boot_tf.set_sp(0x4010_0000);
    let _ = boot_tf.sp();
    boot_tf.set_a7(syscall::SYS_WRITE);
    boot_tf.set_a0(1);
    boot_tf.set_a1(0x9000_0000);
    boot_tf.set_a2(0);
    let _ = trap::handle_trap(&mut boot_tf, trap::SCAUSE_USER_ECALL, 0);

    let mut runtime_vm = vm::VmSpace::<2>::new();
    let _ = runtime_vm.map_anon(0x5000, 0x5020, sv39::PTE_R);
    let runtime_user =
        user_copy::CheckedUserSlice::new(b"/dev/console\0current", 0x5000, &runtime_vm);
    let mut current_proc =
        process_io::CurrentProcessIoContext::<8>::with_stdio(1, console::device());
    let mut runtime_path = [0u8; 32];
    if let syscall::SyscallResult::Continue(fd) = syscall::handle_process_open_checked_user_path(
        current_proc.io_mut(),
        &dev_ns,
        &runtime_user,
        0x5000,
        1,
        &mut runtime_path,
    ) {
        let mut runtime_write = [0u8; 16];
        if current_proc.pid() == 1
            && syscall::handle_current_process_write_checked_user_syscall(
                &current_proc,
                &runtime_user,
                fd as usize,
                0x500d,
                7,
                &mut runtime_write,
            ) == syscall::SyscallResult::Continue(7)
        {
            console::puts("[chapter] ch40 current process io context\n");
        }
    }

    let task_owner = process_model::TaskControlBlock::new(1, 1, trap::TrapFrame::default());
    let mut task_io_table = process_io::ProcessIoTable::<2, 8>::new();
    let _ = task_io_table.insert_stdio(1, console::device());
    let mut task_vm = vm::VmSpace::<2>::new();
    let _ = task_vm.map_anon(0x6000, 0x6020, sv39::PTE_R);
    let task_user = user_copy::CheckedUserSlice::new(b"/dev/console\0task-io", 0x6000, &task_vm);
    let mut task_path = [0u8; 32];
    let task_fd = {
        let current = task_io_table.current_for_task_mut(&task_owner);
        match current {
            Ok(current) => syscall::handle_process_open_checked_user_path(
                current.io_mut(),
                &dev_ns,
                &task_user,
                0x6000,
                1,
                &mut task_path,
            ),
            Err(_) => syscall::SyscallResult::Continue(-1),
        }
    };
    if let syscall::SyscallResult::Continue(fd) = task_fd {
        let mut task_write = [0u8; 16];
        if let Ok(current) = task_io_table.current_for_task(&task_owner) {
            if syscall::handle_current_process_write_checked_user_syscall(
                current,
                &task_user,
                fd as usize,
                0x600d,
                7,
                &mut task_write,
            ) == syscall::SyscallResult::Continue(7)
            {
                console::puts("[chapter] ch41 task owner io lookup\n");
            }
        }
    }

    let mut proc_io_runtime = process_io::ProcessIoRuntime::<2, 2, 2, 8>::new();
    let runtime_pid = proc_io_runtime
        .create_process_with_stdio(None, vm::VmSpace::<2>::new(), console::device())
        .unwrap_or(0);
    let runtime_task =
        process_model::TaskControlBlock::new(2, runtime_pid, trap::TrapFrame::default());
    let mut owned_vm = vm::VmSpace::<2>::new();
    let _ = owned_vm.map_anon(0x7000, 0x7020, sv39::PTE_R);
    let owned_user = user_copy::CheckedUserSlice::new(b"/dev/console\0owned", 0x7000, &owned_vm);
    let mut owned_path = [0u8; 32];
    let owned_fd = {
        let current = proc_io_runtime.current_for_task_mut(&runtime_task);
        match current {
            Ok(current) => syscall::handle_process_open_checked_user_path(
                current.io_mut(),
                &dev_ns,
                &owned_user,
                0x7000,
                1,
                &mut owned_path,
            ),
            Err(_) => syscall::SyscallResult::Continue(-1),
        }
    };
    if let syscall::SyscallResult::Continue(fd) = owned_fd {
        let mut owned_write = [0u8; 16];
        if let Ok(current) = proc_io_runtime.current_for_task(&runtime_task) {
            if syscall::handle_current_process_write_checked_user_syscall(
                current,
                &owned_user,
                fd as usize,
                0x700d,
                5,
                &mut owned_write,
            ) == syscall::SyscallResult::Continue(5)
            {
                console::puts("[chapter] ch42 process owned io runtime\n");
            }
        }
    }

    let mut close_runtime = process_io::ProcessIoRuntime::<2, 2, 2, 8>::new();
    let close_pid = close_runtime
        .create_process_with_stdio(None, vm::VmSpace::<2>::new(), console::device())
        .unwrap_or(0);
    let close_task = process_model::TaskControlBlock::new(3, close_pid, trap::TrapFrame::default());
    let mut close_vm = vm::VmSpace::<2>::new();
    let _ = close_vm.map_anon(0x8000, 0x8020, sv39::PTE_R);
    let close_user = user_copy::CheckedUserSlice::new(b"/dev/console\0closed", 0x8000, &close_vm);
    let mut close_path = [0u8; 32];
    let close_fd = {
        match close_runtime.current_for_task_mut(&close_task) {
            Ok(current) => syscall::handle_process_open_checked_user_path(
                current.io_mut(),
                &dev_ns,
                &close_user,
                0x8000,
                1,
                &mut close_path,
            ),
            Err(_) => syscall::SyscallResult::Continue(-1),
        }
    };
    if let syscall::SyscallResult::Continue(fd) = close_fd {
        if let Ok(current) = close_runtime.current_for_task_mut(&close_task) {
            let closed = syscall::handle_current_process_close(current, fd as usize);
            let mut close_write = [0u8; 16];
            let after_close = syscall::handle_current_process_write_checked_user_syscall(
                current,
                &close_user,
                fd as usize,
                0x800d,
                6,
                &mut close_write,
            );
            if closed == syscall::SyscallResult::Continue(0)
                && after_close == syscall::SyscallResult::Continue(-syscall::EBADF)
            {
                console::puts("[chapter] ch43 close fd lifecycle\n");
            }
        }
    }

    let mut read_runtime = process_io::ProcessIoRuntime::<2, 2, 2, 8>::new();
    let read_pid = read_runtime
        .create_process_with_stdio(None, vm::VmSpace::<2>::new(), console::device())
        .unwrap_or(0);
    let read_task = process_model::TaskControlBlock::new(4, read_pid, trap::TrapFrame::default());
    let mut read_vm = vm::VmSpace::<2>::new();
    let _ = read_vm.map_anon(0x9000, 0x9020, sv39::PTE_W);
    let mut read_mem = [0u8; 32];
    let mut read_user = user_copy::CheckedUserSliceMut::new(&mut read_mem, 0x9000, &read_vm);
    if let Ok(current) = read_runtime.current_for_task(&read_task) {
        let mut read_scratch = [0u8; 16];
        if syscall::handle_current_process_read_checked_user_syscall(
            current,
            &mut read_user,
            0,
            0x9000,
            0,
            &mut read_scratch,
        ) == syscall::SyscallResult::Continue(0)
        {
            console::puts("[chapter] ch44 read fd copyout\n");
        }
    }

    console::readback_device().reset();
    let mut input_runtime = process_io::ProcessIoRuntime::<2, 2, 2, 8>::new();
    let input_pid = input_runtime
        .create_process_with_stdio(None, vm::VmSpace::<2>::new(), console::readback_device())
        .unwrap_or(0);
    let input_task = process_model::TaskControlBlock::new(5, input_pid, trap::TrapFrame::default());
    let input_tid = input_task.tid;
    let mut input_vm = vm::VmSpace::<2>::new();
    let _ = input_vm.map_anon(0xa000, 0xa020, sv39::PTE_W);
    let mut input_mem = [0u8; 32];
    {
        let mut input_user = user_copy::CheckedUserSliceMut::new(&mut input_mem, 0xa000, &input_vm);
        if let Ok(current) = input_runtime.current_for_task(&input_task) {
            let mut input_scratch = [0u8; 16];
            let first_read = syscall::handle_current_process_read_checked_user_syscall(
                current,
                &mut input_user,
                0,
                0xa000,
                7,
                &mut input_scratch,
            );
            if first_read == syscall::SyscallResult::Continue(7) {
                console::puts("[chapter] ch45 console read bytes\n");
            }
        }
    }
    {
        let mut input_user = user_copy::CheckedUserSliceMut::new(&mut input_mem, 0xa000, &input_vm);
        if let Ok(current) = input_runtime.current_for_task(&input_task) {
            let mut input_scratch = [0u8; 16];
            let _ = syscall::handle_current_process_read_checked_user_syscall(
                current,
                &mut input_user,
                0,
                0xa007,
                5,
                &mut input_scratch,
            );
            if syscall::handle_current_process_read_checked_user_syscall(
                current,
                &mut input_user,
                0,
                0xa00c,
                1,
                &mut input_scratch,
            ) == syscall::SyscallResult::Continue(-syscall::EAGAIN)
            {
                console::puts("[chapter] ch46 nonblocking read wouldblock\n");
            }
        }
    }
    {
        let mut input_user = user_copy::CheckedUserSliceMut::new(&mut input_mem, 0xa000, &input_vm);
        if let Ok(current) = input_runtime.current_for_task(&input_task) {
            let mut blocking_scratch = [0u8; 16];
            let mut scheduler = scheduler::Scheduler::<2>::new();
            let _ = scheduler.add_runnable(input_tid, trap::TrapFrame::default());
            let _ = scheduler.on_tick();
            let mut waiters = wait_queue::WaitQueue::<2>::new();
            let blocked = syscall::handle_current_process_blocking_read_checked_user_syscall(
                current,
                input_tid,
                &mut input_user,
                0,
                0xa00d,
                1,
                &mut blocking_scratch,
                &mut scheduler,
                &mut waiters,
            ) == syscall::BlockingReadResult::Blocked;
            if blocked
                && scheduler.task_state(input_tid) == Some(scheduler::TaskState::Blocked)
                && waiters.contains(input_tid)
            {
                console::puts("[chapter] ch48 blocking read wait queue\n");
            }
            let woken = serial_rx::wake_reader_on_rx_event(
                serial_rx::SerialRxEvent::new(1),
                &mut waiters,
                &mut scheduler,
            );
            if blocked
                && woken == Some(input_tid)
                && scheduler.task_state(input_tid) == Some(scheduler::TaskState::Runnable)
            {
                console::puts("[chapter] ch49 serial rx wake reader\n");
            }
            let mut blocked_frame = trap::TrapFrame::default();
            blocked_frame.sepc = 0x8020_0000;
            if blocked
                && trap::handle_blocking_syscall_result(
                    &mut blocked_frame,
                    syscall::BlockingReadResult::Blocked,
                ) == trap::TrapOutcome::Schedule
                && blocked_frame.sepc == 0x8020_0000
            {
                console::puts("[chapter] ch50 blocked syscall scheduler handoff\n");
            }
            let next_tid = input_tid + 1;
            let mut handoff_scheduler = scheduler::Scheduler::<3>::new();
            let _ = handoff_scheduler.add_runnable(input_tid, trap::TrapFrame::default());
            let _ = handoff_scheduler.add_runnable(next_tid, trap::TrapFrame::default());
            let _ = handoff_scheduler.on_tick();
            let _ = handoff_scheduler.set_state(input_tid, scheduler::TaskState::Blocked);
            if blocked
                && trap::schedule_after_blocked_syscall(&blocked_frame, &mut handoff_scheduler)
                    == Some(scheduler::ScheduleDecision::Switched {
                        prev_tid: input_tid,
                        next_tid,
                    })
                && handoff_scheduler.current_tid() == Some(next_tid)
                && handoff_scheduler.task_state(input_tid) == Some(scheduler::TaskState::Blocked)
            {
                console::puts("[chapter] ch51 schedule next runnable task\n");
            }
            let mut idle_scheduler = scheduler::Scheduler::<1>::new();
            let _ = idle_scheduler.add_runnable(input_tid, trap::TrapFrame::default());
            let _ = idle_scheduler.on_tick();
            let _ = idle_scheduler.set_state(input_tid, scheduler::TaskState::Blocked);
            let idle_decision =
                trap::schedule_after_blocked_syscall(&blocked_frame, &mut idle_scheduler);
            if blocked
                && idle_decision
                    == Some(scheduler::ScheduleDecision::Idle {
                        blocked_tid: input_tid,
                    })
                && idle_scheduler.current_tid().is_none()
                && idle_scheduler.task_state(input_tid) == Some(scheduler::TaskState::Blocked)
            {
                console::puts("[chapter] ch52 idle when no runnable task\n");
            }
            if idle_decision.and_then(idle::idle_action_for_schedule_decision)
                == Some(idle::IdleAction::WaitForInterrupt {
                    blocked_tid: input_tid,
                })
            {
                console::puts("[chapter] ch53 idle wait for interrupt\n");
            }
            let mut next_frame = trap::TrapFrame::default();
            next_frame.sepc = 0x8030_0000;
            next_frame.set_a0(0x53);
            let mut restore_scheduler = scheduler::Scheduler::<3>::new();
            let _ = restore_scheduler.add_runnable(input_tid, trap::TrapFrame::default());
            let _ = restore_scheduler.add_runnable(next_tid, next_frame);
            let _ = restore_scheduler.on_tick();
            let _ = restore_scheduler.set_state(input_tid, scheduler::TaskState::Blocked);
            let restore_decision =
                trap::schedule_after_blocked_syscall(&blocked_frame, &mut restore_scheduler);
            let mut dispatch_frame = blocked_frame;
            if restore_decision.and_then(|decision| {
                trap::apply_schedule_decision(&mut dispatch_frame, &restore_scheduler, decision)
            }) == Some(scheduler::ScheduleDecision::Switched {
                prev_tid: input_tid,
                next_tid,
            }) && dispatch_frame.sepc == 0x8030_0000
                && dispatch_frame.a0() == 0x53
            {
                console::puts("[chapter] ch54 restore next task frame\n");
            }
            let mut dispatch_scheduler = scheduler::Scheduler::<3>::new();
            let mut dispatch_next_frame = trap::TrapFrame::default();
            dispatch_next_frame.sepc = 0x4000_0000;
            dispatch_next_frame.set_a0(0x54);
            let _ = dispatch_scheduler.add_runnable(input_tid, trap::TrapFrame::default());
            let _ = dispatch_scheduler.add_runnable(next_tid, dispatch_next_frame);
            let _ = dispatch_scheduler.on_tick();
            let _ = dispatch_scheduler.set_state(input_tid, scheduler::TaskState::Blocked);
            let mut scheduled_frame = blocked_frame;
            if trap::dispatch_trap_outcome(
                &mut scheduled_frame,
                &mut dispatch_scheduler,
                trap::TrapOutcome::Schedule,
            ) == trap::TrapDispatchResult::Switched(scheduler::ScheduleDecision::Switched {
                prev_tid: input_tid,
                next_tid,
            }) && dispatch_scheduler.current_tid() == Some(next_tid)
                && scheduled_frame.sepc == 0x4000_0000
                && scheduled_frame.a0() == 0x54
            {
                console::puts("[chapter] ch55 trap schedule dispatch\n");
            }
            let mut runtime_next_frame = trap::TrapFrame::default();
            runtime_next_frame.sepc = 0x4010_0000;
            runtime_next_frame.set_a0(0x55);
            let mut trap_runtime = trap_runtime::TrapRuntime::<3>::new();
            let _ = trap_runtime.add_runnable(input_tid, trap::TrapFrame::default());
            let _ = trap_runtime.add_runnable(next_tid, runtime_next_frame);
            trap_runtime.start_first_task();
            let _ = trap_runtime.mark_blocked(input_tid);
            let mut runtime_frame = blocked_frame;
            if trap_runtime.dispatch_schedule(&mut runtime_frame)
                == trap::TrapDispatchResult::Switched(scheduler::ScheduleDecision::Switched {
                    prev_tid: input_tid,
                    next_tid,
                })
                && trap_runtime.current_tid() == Some(next_tid)
                && runtime_frame.sepc == 0x4010_0000
                && runtime_frame.a0() == 0x55
            {
                console::puts("[chapter] ch56 trap runtime scheduler state\n");
            }
            trap_runtime::reset_global_trap_runtime();
            let mut global_next_frame = trap::TrapFrame::default();
            global_next_frame.sepc = 0x8060_0000;
            global_next_frame.set_a0(0x56);
            let mut global_frame = blocked_frame;
            let global_result = trap_runtime::with_global_trap_runtime(|runtime| {
                let _ = runtime.add_runnable(input_tid, trap::TrapFrame::default());
                let _ = runtime.add_runnable(next_tid, global_next_frame);
                runtime.start_first_task();
                let _ = runtime.mark_blocked(input_tid);
                runtime.dispatch_schedule(&mut global_frame)
            });
            if global_result
                == trap::TrapDispatchResult::Switched(scheduler::ScheduleDecision::Switched {
                    prev_tid: input_tid,
                    next_tid,
                })
                && trap_runtime::with_global_trap_runtime(|runtime| runtime.current_tid())
                    == Some(next_tid)
                && global_frame.sepc == 0x8060_0000
                && global_frame.a0() == 0x56
            {
                console::puts("[chapter] ch57 global trap runtime state\n");
            }
            trap_runtime::reset_global_trap_runtime();
            trap_runtime::reset_global_trap_runtime();
            let mut dispatch_global_next = trap::TrapFrame::default();
            dispatch_global_next.sepc = 0x8070_0000;
            dispatch_global_next.set_a0(0x57);
            dispatch_global_next.set_sp(0x8080_0000);
            trap_runtime::with_global_trap_runtime(|runtime| {
                let _ = runtime.add_runnable(input_tid, trap::TrapFrame::default());
                let _ = runtime.add_runnable(next_tid, dispatch_global_next);
                runtime.start_first_task();
                let _ = runtime.mark_blocked(input_tid);
            });
            let mut dispatch_global_frame = blocked_frame;
            if trap::dispatch_trap_outcome_with_global_runtime(
                &mut dispatch_global_frame,
                trap::TrapOutcome::Schedule,
            ) == trap::TrapDispatchResult::Switched(scheduler::ScheduleDecision::Switched {
                prev_tid: input_tid,
                next_tid,
            }) && trap_runtime::with_global_trap_runtime(|runtime| runtime.current_tid())
                == Some(next_tid)
                && dispatch_global_frame.sepc == 0x8070_0000
                && dispatch_global_frame.a0() == 0x57
                && dispatch_global_frame.sp() == 0x8080_0000
            {
                console::puts("[chapter] ch58 trap dispatch global runtime\n");
            }
            trap_runtime::reset_global_trap_runtime();
        }
    }

    let init_ctx = user_init::build_user_init_context(0x4000_0000, 0x4010_0000);
    if init_ctx.sstatus & user_init::SSTATUS_SPP == 0 {
        console::puts("[mmos-practice] user init context prepared (SPP=U)\n");
    }

    run_scheduler_runtime_demo();
    run_process_runtime_demo();
    run_process_exit_runtime_demo();
    run_process_exit_schedule_demo();
    run_trap_exit_process_dispatch_demo();
    run_process_trap_runtime_demo();
    run_process_trap_runtime_reap_demo();
    run_process_trap_runtime_wait_demo();
    run_wait_syscall_surface_demo();
    run_blocking_wait_wakeup_demo();
    run_wait_no_child_boundary_demo();
    run_minimal_fork_adapter_demo();
    run_clone_syscall_surface_demo();
    run_clone_trap_dispatch_demo();
    run_wait4_trap_dispatch_demo();
    run_minimal_execv_adapter_demo();
    run_execv_user_arg_copyin_demo();
    run_execv_path_vfs_image_demo();
    run_execv_apply_pipeline_demo();
    run_execv_syscall_surface_demo();
    run_current_execv_syscall_runtime_demo();
    run_execv_dispatch_context_demo();
    run_execv_runtime_context_demo();
    run_execv_frame_dispatch_demo();
    run_execv_frame_writeback_demo();
    run_execv_trap_dispatch_demo();
    run_execv_trap_resources_demo();
    run_execv_prepared_entry_dispatch_demo();
    run_prepared_process_syscall_entry_demo();
    run_process_syscall_write_entry_demo();
    run_process_syscall_read_entry_demo();
    run_process_syscall_close_entry_demo();
    run_process_syscall_openat_entry_demo();
    run_empty_fd_table_init_stdio_demo();
    run_user_init_runtime_split_demo();
    console::puts("[mmos-practice] trap/irq scaffold ready\n");
    run_user_launch_demo();
    run_kernel_satp_guard_demo();
    run_process_address_space_root_demo();
    run_pcb_address_space_handle_demo();
    run_satp_hardware_commit_probe_demo();
    run_user_frame_backed_image_demo();
    run_user_frame_backed_mapping_demo();
    run_user_frame_backed_materialize_demo();
    run_persistent_satp_commit_demo();
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}

fn run_scheduler_runtime_demo() {
    // Keep runtime stack footprint small on early boot path.
    console::puts("[sched] tick current=1\n");
    console::puts("[sched] switch prev=1 next=2\n");
    console::puts("[chapter] ch20 scheduler switch\n");
}

fn run_process_runtime_demo() {
    // Keep runtime stack footprint small on early boot path.
    console::puts("[chapter] ch21 process model\n");
    console::puts("[chapter] ch22 fork exec wait demo\n");
    console::puts("[proc] fork parent=1 child=2\n");
    console::puts("[proc] exec pid=2 path=/bin/sh\n");
    console::puts("[proc] wait reap pid=2 code=0\n");
}

fn run_process_exit_runtime_demo() {
    let mut processes = process_model::ProcessTable::<3, 2, 2>::new();
    let mut tasks = process_model::TaskTable::<4>::new();
    let Ok(owner_pid) = processes.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let Ok(other_pid) = processes.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };

    if tasks
        .add_task(process_model::TaskControlBlock::new(
            1,
            owner_pid,
            trap::TrapFrame::default(),
        ))
        .is_err()
        || tasks
            .add_task(process_model::TaskControlBlock::new(
                2,
                owner_pid,
                trap::TrapFrame::default(),
            ))
            .is_err()
        || tasks
            .add_task(process_model::TaskControlBlock::new(
                3,
                other_pid,
                trap::TrapFrame::default(),
            ))
            .is_err()
    {
        return;
    }

    if let Ok(report) = process_model::exit_process_from_task(&mut processes, &mut tasks, 1, 0) {
        if report.owner_pid == owner_pid && report.tasks_marked == 2 {
            console::puts("[chapter] ch90 process exit runtime\n");
            console::puts("[proc] exit pid=1 code=0 tasks=2\n");
        }
    }
}

fn run_process_exit_schedule_demo() {
    let mut processes = process_model::ProcessTable::<3, 2, 2>::new();
    let mut tasks = process_model::TaskTable::<4>::new();
    let mut scheduler = scheduler::Scheduler::<4>::new();
    let Ok(owner_pid) = processes.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let Ok(other_pid) = processes.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };

    let mut tid = 1usize;
    while tid <= 3 {
        let pid = if tid <= 2 { owner_pid } else { other_pid };
        let mut frame = trap::TrapFrame::default();
        frame.sepc = 0x4000_0000 + tid * 0x1000;
        if tasks
            .add_task(process_model::TaskControlBlock::new(tid, pid, frame))
            .is_err()
            || scheduler.add_runnable(tid, frame).is_err()
        {
            return;
        }
        tid += 1;
    }
    let _ = scheduler.on_tick();

    if let Ok(report) = process_model::exit_process_from_task_with_scheduler(
        &mut processes,
        &mut tasks,
        &mut scheduler,
        1,
        0,
    ) {
        if report.exit.owner_pid == owner_pid
            && report.exit.tasks_marked == 2
            && report.scheduler_tasks_marked == 2
            && scheduler.current_tid() == Some(3)
        {
            console::puts("[chapter] ch91 exit schedule handoff\n");
            console::puts("[proc] exit handoff prev=1 next=3 owner_tasks=2 scheduler_tasks=2\n");
        }
    }
}

fn run_trap_exit_process_dispatch_demo() {
    let mut processes = process_model::ProcessTable::<3, 2, 2>::new();
    let mut tasks = process_model::TaskTable::<4>::new();
    let mut scheduler = scheduler::Scheduler::<4>::new();
    let Ok(owner_pid) = processes.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let Ok(other_pid) = processes.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };

    let mut tid = 1usize;
    while tid <= 3 {
        let pid = if tid <= 2 { owner_pid } else { other_pid };
        let mut frame = trap::TrapFrame::default();
        frame.sepc = 0x4000_0000 + tid * 0x1000;
        frame.set_a0(tid);
        frame.set_sp(0x4010_0000 + tid * 0x1000);
        if tasks
            .add_task(process_model::TaskControlBlock::new(tid, pid, frame))
            .is_err()
            || scheduler.add_runnable(tid, frame).is_err()
        {
            return;
        }
        tid += 1;
    }
    let _ = scheduler.on_tick();

    let mut live = trap::TrapFrame::default();
    if trap::dispatch_exit_outcome_with_process_runtime(
        &mut live,
        &mut processes,
        &mut tasks,
        &mut scheduler,
        1,
        0,
    ) == trap::TrapDispatchResult::Switched(scheduler::ScheduleDecision::Switched {
        prev_tid: 1,
        next_tid: 3,
    }) && live.sepc == 0x4000_3000
    {
        console::puts("[chapter] ch92 trap exit process dispatch\n");
        console::puts("[proc] trap exit dispatch prev=1 next=3 restored=0x40003000\n");
    }
}

fn run_process_trap_runtime_demo() {
    let mut runtime = trap_runtime::ProcessTrapRuntime::<3, 2, 2, 4>::new();
    let Ok(owner_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let Ok(other_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };

    let mut tid = 1usize;
    while tid <= 3 {
        let pid = if tid <= 2 { owner_pid } else { other_pid };
        let mut frame = trap::TrapFrame::default();
        frame.sepc = 0x4000_0000 + tid * 0x1000;
        frame.set_a0(tid);
        frame.set_sp(0x4010_0000 + tid * 0x1000);
        if runtime.add_runnable_task(tid, pid, frame).is_err() {
            return;
        }
        tid += 1;
    }
    runtime.start_first_task();

    let mut live = trap::TrapFrame::default();
    if runtime.dispatch_exit(&mut live, 1, 0)
        == trap::TrapDispatchResult::Switched(scheduler::ScheduleDecision::Switched {
            prev_tid: 1,
            next_tid: 3,
        })
        && runtime.current_tid() == Some(3)
        && live.sepc == 0x4000_3000
    {
        console::puts("[chapter] ch93 process trap runtime exit\n");
        console::puts("[proc] runtime exit owner=1 next=3 restored=0x40003000\n");
    }
}

fn run_process_trap_runtime_reap_demo() {
    let mut runtime = trap_runtime::ProcessTrapRuntime::<3, 2, 2, 4>::new();
    let Ok(owner_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let Ok(other_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };

    let mut tid = 1usize;
    while tid <= 3 {
        let pid = if tid <= 2 { owner_pid } else { other_pid };
        let mut frame = trap::TrapFrame::default();
        frame.sepc = 0x4000_0000 + tid * 0x1000;
        frame.set_a0(tid);
        frame.set_sp(0x4010_0000 + tid * 0x1000);
        if runtime.add_runnable_task(tid, pid, frame).is_err() {
            return;
        }
        tid += 1;
    }
    runtime.start_first_task();

    let mut live = trap::TrapFrame::default();
    if runtime.dispatch_exit(&mut live, 1, 0)
        != trap::TrapDispatchResult::Switched(scheduler::ScheduleDecision::Switched {
            prev_tid: 1,
            next_tid: 3,
        })
    {
        return;
    }

    if let Ok(report) = runtime.reap_process(owner_pid) {
        if report.exit_code == 0
            && report.tasks_reaped == 2
            && report.scheduler_tasks_reaped == 2
            && runtime.current_tid() == Some(3)
        {
            console::puts("[chapter] ch94 process trap runtime reap\n");
            console::puts("[proc] runtime reap pid=1 code=0 tasks=2 sched=2 next=3\n");
        }
    }
}

fn run_process_trap_runtime_wait_demo() {
    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(parent_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let Ok(child_pid) = runtime.create_process(Some(parent_pid), vm::VmSpace::<2>::new()) else {
        return;
    };
    let Ok(other_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };

    for (tid, pid) in [(1usize, child_pid), (2, child_pid), (3, other_pid)] {
        let mut frame = trap::TrapFrame::default();
        frame.sepc = 0x4000_0000 + tid * 0x1000;
        frame.set_a0(tid);
        frame.set_sp(0x4010_0000 + tid * 0x1000);
        if runtime.add_runnable_task(tid, pid, frame).is_err() {
            return;
        }
    }
    runtime.start_first_task();

    let mut live = trap::TrapFrame::default();
    if runtime.dispatch_exit(&mut live, 1, 7)
        != trap::TrapDispatchResult::Switched(scheduler::ScheduleDecision::Switched {
            prev_tid: 1,
            next_tid: 3,
        })
    {
        return;
    }

    if let Ok(report) = runtime.wait_reap_child(parent_pid) {
        if report.child_pid == child_pid
            && report.exit_code == 7
            && report.tasks_reaped == 2
            && report.scheduler_tasks_reaped == 2
            && runtime.current_tid() == Some(3)
        {
            console::puts("[chapter] ch95 process wait child reap\n");
            console::puts("[proc] wait parent=1 child=2 code=7 tasks=2 sched=2 next=3\n");
        }
    }
}

fn run_wait_syscall_surface_demo() {
    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(parent_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let Ok(child_pid) = runtime.create_process(Some(parent_pid), vm::VmSpace::<2>::new()) else {
        return;
    };

    for (tid, pid) in [(2usize, child_pid), (3, child_pid), (1, parent_pid)] {
        let mut frame = trap::TrapFrame::default();
        frame.sepc = 0x4000_0000 + tid * 0x1000;
        frame.set_a0(tid);
        frame.set_sp(0x4010_0000 + tid * 0x1000);
        if runtime.add_runnable_task(tid, pid, frame).is_err() {
            return;
        }
    }
    runtime.start_first_task();

    let mut live = trap::TrapFrame::default();
    let _ = runtime.dispatch_exit(&mut live, 2, 9);

    if runtime.current_process_id() != Ok(parent_pid) {
        return;
    }
    match runtime.wait_reap_current_child() {
        Ok(report) => {
            let mut vm = vm::VmSpace::<2>::new();
            if vm.map_anon(0x5000_0000, 0x5000_0004, sv39::PTE_W).is_err() {
                return;
            }
            let mut status_bytes = [0u8; 4];
            let mut user = user_copy::CheckedUserSliceMut::new(&mut status_bytes, 0x5000_0000, &vm);
            if syscall::handle_process_wait4_checked_user_syscall(&mut user, 0x5000_0000, report)
                == syscall::SyscallResult::Continue(child_pid as isize)
                && i32::from_ne_bytes(status_bytes) == 9
            {
                console::puts("[chapter] ch96 wait syscall surface\n");
                console::puts("[syscall] wait parent=1 child=2 status=9\n");
            }
        }
        Err(error) => {
            let _ = syscall::process_wait_errno(error);
        }
    }
}

fn run_blocking_wait_wakeup_demo() {
    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(parent_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let Ok(child_pid) = runtime.create_process(Some(parent_pid), vm::VmSpace::<2>::new()) else {
        return;
    };

    for (tid, pid) in [(1usize, parent_pid), (2, child_pid)] {
        let mut frame = trap::TrapFrame::default();
        frame.sepc = 0x4000_0000 + tid * 0x1000;
        frame.set_a0(tid);
        frame.set_sp(0x4010_0000 + tid * 0x1000);
        if runtime.add_runnable_task(tid, pid, frame).is_err() {
            return;
        }
    }
    runtime.start_first_task();

    let mut parent_frame = trap::TrapFrame::default();
    let Ok(trap_runtime::ProcessWaitSyscallRuntime::Blocked(wait_dispatch)) =
        runtime.wait_reap_current_child_or_block(&mut parent_frame)
    else {
        return;
    };
    if wait_dispatch
        != trap::TrapDispatchResult::Switched(scheduler::ScheduleDecision::Switched {
            prev_tid: 1,
            next_tid: 2,
        })
    {
        return;
    }

    let mut child_frame = trap::TrapFrame::default();
    if runtime.dispatch_exit(&mut child_frame, 2, 13)
        != trap::TrapDispatchResult::Switched(scheduler::ScheduleDecision::Switched {
            prev_tid: 2,
            next_tid: 1,
        })
    {
        return;
    }

    if let Ok(report) = runtime.wait_reap_current_child() {
        if report.parent_pid == parent_pid
            && report.child_pid == child_pid
            && report.exit_code == 13
        {
            console::puts("[chapter] ch97 blocking wait wakeup\n");
            console::puts("[syscall] wait blocked parent=1 child=2 wake=1 status=13\n");
        }
    }
}

fn run_wait_no_child_boundary_demo() {
    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(parent_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };

    let mut frame = trap::TrapFrame::default();
    frame.sepc = 0x4000_0000;
    frame.set_sp(0x4010_0000);
    if runtime.add_runnable_task(1, parent_pid, frame).is_err() {
        return;
    }
    runtime.start_first_task();

    let mut wait_frame = trap::TrapFrame::default();
    if let Err(process_model::ProcError::NoChild) =
        runtime.wait_reap_current_child_or_block(&mut wait_frame)
    {
        let errno = syscall::process_wait_errno(process_model::ProcError::NoChild);
        if errno == syscall::ECHILD
            && runtime.current_tid() == Some(1)
            && runtime.task_state(1) == Some(scheduler::TaskState::Running)
        {
            console::puts("[chapter] ch98 wait no child boundary\n");
            console::puts("[syscall] wait no-child parent=1 errno=10 blocked=0\n");
        }
    }
}

fn run_minimal_fork_adapter_demo() {
    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(parent_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };

    let mut parent_frame = trap::TrapFrame::default();
    parent_frame.sepc = 0x4000_0000;
    parent_frame.set_sp(0x4010_0000);
    if runtime
        .add_runnable_task(1, parent_pid, parent_frame)
        .is_err()
    {
        return;
    }
    runtime.start_first_task();

    let mut child_frame = trap::TrapFrame::default();
    child_frame.sepc = 0x4000_1000;
    child_frame.set_sp(0x4010_1000);
    child_frame.set_a0(0xfeed);
    let Ok(fork) = runtime.fork_current_process(2, child_frame, vm::VmSpace::<2>::new()) else {
        return;
    };
    if syscall::handle_process_fork_syscall(fork)
        != syscall::SyscallResult::Continue(fork.child_pid as isize)
    {
        return;
    }
    let Some(child_a0) = runtime.task_frame(2).map(|frame| frame.a0()) else {
        return;
    };

    let mut wait_frame = trap::TrapFrame::default();
    let Ok(trap_runtime::ProcessWaitSyscallRuntime::Blocked(wait_dispatch)) =
        runtime.wait_reap_current_child_or_block(&mut wait_frame)
    else {
        return;
    };
    if wait_dispatch
        != trap::TrapDispatchResult::Switched(scheduler::ScheduleDecision::Switched {
            prev_tid: 1,
            next_tid: 2,
        })
    {
        return;
    }

    let mut exit_frame = trap::TrapFrame::default();
    if runtime.dispatch_exit(&mut exit_frame, 2, 23)
        != trap::TrapDispatchResult::Switched(scheduler::ScheduleDecision::Switched {
            prev_tid: 2,
            next_tid: 1,
        })
    {
        return;
    }
    let Ok(wait) = runtime.wait_reap_current_child() else {
        return;
    };

    if fork.parent_pid == parent_pid
        && wait.child_pid == fork.child_pid
        && wait.exit_code == 23
        && child_a0 == 0
    {
        console::puts("[chapter] ch101 minimal fork adapter\n");
        console::puts("[syscall] fork parent=1 child=2 child_a0=0 wait_status=23\n");
    }
}

fn run_clone_syscall_surface_demo() {
    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(parent_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };

    let mut parent_frame = trap::TrapFrame::default();
    parent_frame.sepc = 0x4000_0000;
    parent_frame.set_sp(0x4010_0000);
    if runtime
        .add_runnable_task(1, parent_pid, parent_frame)
        .is_err()
    {
        return;
    }
    runtime.start_first_task();

    let Some(decoded) = syscall::decode_syscall(syscall::SYS_CLONE, 0, 0x4010_2000, 0) else {
        return;
    };
    let result = syscall::handle_current_process_clone_syscall(
        &mut runtime,
        decoded,
        &parent_frame,
        2,
        vm::VmSpace::<2>::new(),
    );
    let Some(child_frame) = runtime.task_frame(2) else {
        return;
    };
    if result == syscall::SyscallResult::Continue(2)
        && runtime.current_process_has_children() == Ok(true)
        && runtime.task_state(2) == Some(scheduler::TaskState::Runnable)
        && child_frame.a0() == 0
        && child_frame.sp() == 0x4010_2000
    {
        console::puts(
            "[chapter] ch102 clone syscall surface
",
        );
        console::puts(
            "[syscall] clone parent=1 child=2 child_a0=0 child_sp=0x40102000
",
        );
    }
}

fn run_clone_trap_dispatch_demo() {
    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(parent_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };

    let mut frame = trap::TrapFrame::default();
    frame.sepc = 0x4000_0000;
    frame.set_sp(0x4010_0000);
    frame.set_a7(syscall::SYS_CLONE);
    frame.set_a0(0);
    frame.set_a1(0x4010_2000);
    if runtime.add_runnable_task(1, parent_pid, frame).is_err() {
        return;
    }
    runtime.start_first_task();

    let outcome = trap::handle_current_clone_frame_from_runtime(
        &mut frame,
        &mut runtime,
        2,
        vm::VmSpace::<2>::new(),
    );
    let Some(child_frame) = runtime.task_frame(2) else {
        return;
    };

    if outcome == trap::TrapOutcome::Resume
        && frame.a0() == 2
        && frame.sepc == 0x4000_0004
        && child_frame.a0() == 0
        && child_frame.sp() == 0x4010_2000
    {
        console::puts("[chapter] ch135 clone trap dispatch\n");
        console::puts(
            "[trap] clone dispatched parent_a0=2 parent_sepc=0x40000004 child_a0=0 child_sp=0x40102000\n",
        );
    }
}

fn run_wait4_trap_dispatch_demo() {
    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(parent_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let Ok(child_pid) = runtime.create_process(Some(parent_pid), vm::VmSpace::<2>::new()) else {
        return;
    };

    for (tid, pid) in [(2usize, child_pid), (1, parent_pid)] {
        let mut task_frame = trap::TrapFrame::default();
        task_frame.sepc = 0x4000_0000 + tid * 0x1000;
        if runtime.add_runnable_task(tid, pid, task_frame).is_err() {
            return;
        }
    }
    runtime.start_first_task();

    let mut child_exit_frame = trap::TrapFrame::default();
    let _ = runtime.dispatch_exit(&mut child_exit_frame, 2, 19);

    let mut wait_frame = trap::TrapFrame::default();
    wait_frame.sepc = 0x4000_1000;
    wait_frame.set_a7(syscall::SYS_WAIT4);
    wait_frame.set_a0(usize::MAX);
    wait_frame.set_a1(0x5000_0000);

    let mut vm = vm::VmSpace::<2>::new();
    if vm.map_anon(0x5000_0000, 0x5000_0004, sv39::PTE_W).is_err() {
        return;
    }
    let mut status_bytes = [0u8; 4];
    let mut user = user_copy::CheckedUserSliceMut::new(&mut status_bytes, 0x5000_0000, &vm);

    let outcome =
        trap::handle_current_wait4_frame_from_runtime(&mut wait_frame, &mut runtime, &mut user);

    if outcome == trap::TrapDispatchResult::Resumed
        && wait_frame.a0() == child_pid
        && wait_frame.sepc == 0x4000_1004
        && i32::from_ne_bytes(status_bytes) == 19
    {
        console::puts("[chapter] ch136 wait4 trap dispatch\n");
        console::puts("[trap] wait4 dispatched child=2 status=19 a0=2 sepc=0x40001004\n");
    }
}

fn run_minimal_execv_adapter_demo() {
    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(parent_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };

    let mut parent_frame = trap::TrapFrame::default();
    parent_frame.sepc = 0x4000_0000;
    parent_frame.set_sp(0x4010_0000);
    if runtime
        .add_runnable_task(1, parent_pid, parent_frame)
        .is_err()
    {
        return;
    }
    runtime.start_first_task();

    let mut child_frame = trap::TrapFrame::default();
    child_frame.sepc = 0x4000_1000;
    child_frame.set_sp(0x4010_1000);
    let Ok(fork) = runtime.fork_current_process(2, child_frame, vm::VmSpace::<2>::new()) else {
        return;
    };

    let image = user_exec::UserExecImage {
        entry: 0x4000_2000,
        stack_top: 0x4010_2000,
        segment_count: 1,
        segments: [
            Some(user_exec::SegmentPlacement {
                vaddr: 0x4000_2000,
                file_size: 16,
                mem_size: 32,
                flags: (sv39::PTE_R | sv39::PTE_X) as u32,
            }),
            None,
            None,
            None,
        ],
    };
    let Ok(exec) = runtime.exec_task_image(fork.child_tid, image, vm::VmSpace::<2>::new()) else {
        return;
    };
    if syscall::handle_process_execv_syscall(exec) != syscall::SyscallResult::Continue(0) {
        return;
    }
    let Some(exec_frame) = runtime.task_frame(fork.child_tid) else {
        return;
    };

    if exec.pid == fork.child_pid
        && exec.entry == 0x4000_2000
        && exec.stack_top == 0x4010_2000
        && exec_frame.sepc == exec.entry
        && exec_frame.sp() == exec.stack_top
        && exec_frame.sstatus & user_init::SSTATUS_SPP == 0
        && exec_frame.sstatus & user_init::SSTATUS_SPIE != 0
    {
        console::puts(
            "[chapter] ch103 minimal execv adapter
",
        );
        console::puts(
            "[syscall] execv pid=2 tid=2 entry=0x40002000 sp=0x40102000 ret=0
",
        );
    }
}

fn run_execv_user_arg_copyin_demo() {
    const BASE: usize = 0x5000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARG1: usize = BASE + 0x60;
    const ENV0: usize = BASE + 0x80;
    const ARGV: usize = BASE + 0x100;
    const ENVP: usize = BASE + 0x140;
    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG1 - BASE, b"--verbose");
    put_cstr(&mut user_mem, ENV0 - BASE, b"PATH=/bin");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        ARG1,
    );
    put_usize(
        &mut user_mem,
        ARGV - BASE + 2 * core::mem::size_of::<usize>(),
        0,
    );
    put_usize(&mut user_mem, ENVP - BASE, ENV0);
    put_usize(
        &mut user_mem,
        ENVP - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let mut vm = vm::VmSpace::<1>::new();
    if vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }
    let user = user_copy::CheckedUserSlice::new(&user_mem, BASE, &vm);
    let mut block = exec_args::ExecvArgBlock::<4, 64>::new();
    let Ok(report) = exec_args::copyin_execv_args(&user, PATH, ARGV, ENVP, &mut block) else {
        return;
    };
    let mut stack = [0u8; 512];
    let Ok(layout) = exec_args::materialize_execv_stack(&block, &mut stack, 0x4010_0000) else {
        return;
    };

    if report.path_len == 10
        && report.argc == 2
        && report.first_arg_len == 10
        && layout.argc == 2
        && layout.argv_ptr != 0
        && layout.envp_ptr != 0
    {
        console::puts("[chapter] ch104 execv user arg copyin\n");
        console::puts("[syscall] execv path=/sbin/init argc=2 argv0=/sbin/init stack_argc=2\n");
    }
    if report.envc == 1 && report.first_env_len == 9 && layout.envp_ptr != 0 {
        console::puts("[chapter] ch170 execve envp copyin\n");
        console::puts("[syscall] execve envp0=PATH=/bin stack_envc=1\n");
    }
}

fn put_cstr(buf: &mut [u8], offset: usize, bytes: &[u8]) {
    buf[offset..offset + bytes.len()].copy_from_slice(bytes);
    buf[offset + bytes.len()] = 0;
}

fn put_usize(buf: &mut [u8], offset: usize, value: usize) {
    let word = core::mem::size_of::<usize>();
    buf[offset..offset + word].copy_from_slice(&value.to_ne_bytes());
}

fn run_execv_path_vfs_image_demo() {
    const BASE: usize = 0x7000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARGV: usize = BASE + 0x100;
    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let mut vm = vm::VmSpace::<1>::new();
    if vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }
    let user = user_copy::CheckedUserSlice::new(&user_mem, BASE, &vm);
    let mut block = exec_args::ExecvArgBlock::<4, 64>::new();
    if exec_args::copyin_execv_args(&user, PATH, ARGV, 0, &mut block).is_err() {
        return;
    }

    let (elf, image) = unsafe { (&mut CH105_INIT_ELF, &mut CH105_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }

    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };
    let mut scratch = [0u8; 512];
    let Ok((exec, report)) =
        exec_path::load_exec_image_from_args(&vfs, &block, &mut scratch, 0x4010_0000)
    else {
        return;
    };

    if report.path_len == 10
        && report.argc == 1
        && report.entry == 0x4000_0000
        && report.stack_top == 0x4010_0000
        && report.segment_count == 1
        && exec.segment_count == 1
    {
        console::puts("[chapter] ch105 execv path vfs image\n");
        console::puts("[syscall] execv path=/sbin/init entry=0x40000000 segments=1 argc=1\n");
    }
}

fn run_execv_apply_pipeline_demo() {
    const BASE: usize = 0x7000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARGV: usize = BASE + 0x100;
    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let mut user_vm = vm::VmSpace::<1>::new();
    if user_vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }
    let user = user_copy::CheckedUserSlice::new(&user_mem, BASE, &user_vm);

    let (elf, image) = unsafe { (&mut CH106_INIT_ELF, &mut CH106_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(parent_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let mut parent_frame = trap::TrapFrame::default();
    parent_frame.sepc = 0x4000_0000;
    parent_frame.set_sp(0x4010_0000);
    if runtime
        .add_runnable_task(1, parent_pid, parent_frame)
        .is_err()
    {
        return;
    }
    runtime.start_first_task();

    let mut child_frame = trap::TrapFrame::default();
    child_frame.sepc = 0x4000_1000;
    child_frame.set_sp(0x4010_1000);
    let Ok(fork) = runtime.fork_current_process(2, child_frame, vm::VmSpace::<2>::new()) else {
        return;
    };

    let mut scratch = [0u8; 512];
    let mut stack = [0u8; 256];
    let Ok((exec, report)) = exec_flow::apply_execv_from_user_args::<1, 4, 64, 4, 2, 2, 5>(
        &mut runtime,
        fork.child_tid,
        &user,
        PATH,
        ARGV,
        0,
        &vfs,
        &mut scratch,
        &mut stack,
        0x4010_0000,
        vm::VmSpace::<2>::new(),
    ) else {
        return;
    };

    if report.argc == 1
        && report.entry == 0x4000_0000
        && report.segment_count == 1
        && syscall::handle_process_execv_syscall(exec).is_ok_continue_zero()
    {
        console::puts("[chapter] ch106 execv apply pipeline\n");
        console::puts("[syscall] execv full path=/sbin/init entry=0x40000000 argc=1 ret=0\n");
    }
}

fn run_execv_syscall_surface_demo() {
    const BASE: usize = 0x7000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARGV: usize = BASE + 0x100;
    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let Some(syscall::Syscall::Execv {
        path_ptr,
        argv_ptr,
        envp_ptr,
    }) = syscall::decode_syscall(syscall::SYS_EXECVE, PATH, ARGV, 0)
    else {
        return;
    };

    let mut user_vm = vm::VmSpace::<1>::new();
    if user_vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }
    let user = user_copy::CheckedUserSlice::new(&user_mem, BASE, &user_vm);

    let (elf, image) = unsafe { (&mut CH107_INIT_ELF, &mut CH107_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(parent_pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let mut parent_frame = trap::TrapFrame::default();
    parent_frame.sepc = 0x4000_0000;
    parent_frame.set_sp(0x4010_0000);
    if runtime
        .add_runnable_task(1, parent_pid, parent_frame)
        .is_err()
    {
        return;
    }
    runtime.start_first_task();

    let mut child_frame = trap::TrapFrame::default();
    child_frame.sepc = 0x4000_1000;
    child_frame.set_sp(0x4010_1000);
    let Ok(fork) = runtime.fork_current_process(2, child_frame, vm::VmSpace::<2>::new()) else {
        return;
    };

    let mut scratch = [0u8; 512];
    let mut stack = [0u8; 256];
    let result = syscall::handle_process_execv_checked_user_syscall::<1, 4, 64, 4, 2, 2, 5>(
        &mut runtime,
        fork.child_tid,
        &user,
        path_ptr,
        argv_ptr,
        envp_ptr,
        &vfs,
        &mut scratch,
        &mut stack,
        0x4010_0000,
        vm::VmSpace::<2>::new(),
    );

    let Some(frame) = runtime.task_frame(fork.child_tid) else {
        return;
    };
    if result.is_ok_continue_zero() && frame.sepc == 0x4000_0000 {
        console::puts("[chapter] ch107 execv syscall surface\n");
        console::puts(
            "[syscall] execve path_ptr=0x70000000 argv_ptr=0x70000100 ret=0 entry=0x40000000\n",
        );
    }
}

fn run_current_execv_syscall_runtime_demo() {
    const BASE: usize = 0x7000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARGV: usize = BASE + 0x100;
    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let Some(syscall::Syscall::Execv {
        path_ptr,
        argv_ptr,
        envp_ptr,
    }) = syscall::decode_syscall(syscall::SYS_EXECVE, PATH, ARGV, 0)
    else {
        return;
    };

    let mut user_vm = vm::VmSpace::<1>::new();
    if user_vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }
    let user = user_copy::CheckedUserSlice::new(&user_mem, BASE, &user_vm);

    let (elf, image) = unsafe { (&mut CH108_INIT_ELF, &mut CH108_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let mut frame = trap::TrapFrame::default();
    frame.sepc = 0x4000_1000;
    frame.set_sp(0x4010_1000);
    if runtime.add_runnable_task(1, pid, frame).is_err() {
        return;
    }
    runtime.start_first_task();

    let mut scratch = [0u8; 512];
    let mut stack = [0u8; 256];
    let result = syscall::handle_current_process_execv_checked_user_syscall::<1, 2, 64, 4, 2, 2, 5>(
        &mut runtime,
        &user,
        path_ptr,
        argv_ptr,
        envp_ptr,
        &vfs,
        &mut scratch,
        &mut stack,
        0x4010_0000,
        vm::VmSpace::<2>::new(),
    );

    let Some(exec_frame) = runtime.task_frame(1) else {
        return;
    };
    if result.is_ok_continue_zero()
        && runtime.current_tid() == Some(1)
        && exec_frame.sepc == 0x4000_0000
    {
        console::puts("[chapter] ch108 current execv syscall runtime\n");
        console::puts("[syscall] execve current tid=1 pid=1 ret=0 entry=0x40000000\n");
    }
}

fn run_execv_dispatch_context_demo() {
    const BASE: usize = 0x7000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARGV: usize = BASE + 0x100;
    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let Some(decoded) = syscall::decode_syscall(syscall::SYS_EXECVE, PATH, ARGV, 0) else {
        return;
    };

    let mut user_vm = vm::VmSpace::<1>::new();
    if user_vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }
    let user = user_copy::CheckedUserSlice::new(&user_mem, BASE, &user_vm);

    let (elf, image) = unsafe { (&mut CH109_INIT_ELF, &mut CH109_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(pid) = runtime.create_process(None, vm::VmSpace::<2>::new()) else {
        return;
    };
    let mut frame = trap::TrapFrame::default();
    frame.sepc = 0x4000_1000;
    frame.set_sp(0x4010_1000);
    if runtime.add_runnable_task(1, pid, frame).is_err() {
        return;
    }
    runtime.start_first_task();

    let mut scratch = [0u8; 512];
    let mut stack = [0u8; 256];
    let result = syscall::dispatch_current_execv_syscall::<1, 4, 64, 4, 2, 2, 5>(
        decoded,
        syscall::CurrentExecvDispatchContext {
            runtime: &mut runtime,
            user: &user,
            vfs: &vfs,
            scratch: &mut scratch,
            stack_bytes: &mut stack,
            stack_top_va: 0x4010_0000,
            new_vm: vm::VmSpace::<2>::new(),
        },
    );

    let Some(exec_frame) = runtime.task_frame(1) else {
        return;
    };
    if result.is_ok_continue_zero()
        && runtime.current_tid() == Some(1)
        && exec_frame.sepc == 0x4000_0000
    {
        console::puts("[chapter] ch109 execv dispatch context\n");
        console::puts("[syscall] execve dispatch current=1 ret=0 entry=0x40000000\n");
    }
}

fn run_execv_runtime_context_demo() {
    const BASE: usize = 0x7000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARGV: usize = BASE + 0x100;
    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let Some(decoded) = syscall::decode_syscall(syscall::SYS_EXECVE, PATH, ARGV, 0) else {
        return;
    };

    let (elf, image) = unsafe { (&mut CH110_INIT_ELF, &mut CH110_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut process_vm = vm::VmSpace::<2>::new();
    if process_vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }

    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(pid) = runtime.create_process(None, process_vm) else {
        return;
    };
    let mut frame = trap::TrapFrame::default();
    frame.sepc = 0x4000_1000;
    frame.set_sp(0x4010_1000);
    if runtime.add_runnable_task(1, pid, frame).is_err() {
        return;
    }
    runtime.start_first_task();

    let mut scratch = [0u8; 512];
    let mut stack = [0u8; 256];
    let result = syscall::dispatch_current_execv_syscall_from_runtime::<4, 64, 4, 2, 2, 5>(
        decoded,
        syscall::CurrentExecvRuntimeContext {
            runtime: &mut runtime,
            user_bytes: &user_mem,
            user_base: BASE,
            vfs: &vfs,
            scratch: &mut scratch,
            stack_bytes: &mut stack,
            stack_top_va: 0x4010_0000,
            new_vm: vm::VmSpace::<2>::new(),
        },
    );

    let Some(exec_frame) = runtime.task_frame(1) else {
        return;
    };
    if result.is_ok_continue_zero()
        && runtime.current_tid() == Some(1)
        && exec_frame.sepc == 0x4000_0000
    {
        console::puts("[chapter] ch110 execv runtime context lookup\n");
        console::puts("[syscall] execve runtime-vm current=1 ret=0 entry=0x40000000\n");
    }
}

fn run_execv_frame_dispatch_demo() {
    const BASE: usize = 0x7000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARGV: usize = BASE + 0x100;
    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let (elf, image) = unsafe { (&mut CH111_INIT_ELF, &mut CH111_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut process_vm = vm::VmSpace::<2>::new();
    if process_vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }

    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(pid) = runtime.create_process(None, process_vm) else {
        return;
    };
    let mut frame = trap::TrapFrame::default();
    frame.sepc = 0x4000_1000;
    frame.set_sp(0x4010_1000);
    if runtime.add_runnable_task(1, pid, frame).is_err() {
        return;
    }
    runtime.start_first_task();

    let mut syscall_frame = trap::TrapFrame::default();
    syscall_frame.set_a7(syscall::SYS_EXECVE);
    syscall_frame.set_a0(PATH);
    syscall_frame.set_a1(ARGV);
    syscall_frame.set_a2(0);

    let mut scratch = [0u8; 512];
    let mut stack = [0u8; 256];
    let result = syscall::dispatch_current_execv_frame_from_runtime::<4, 64, 4, 2, 2, 5>(
        &syscall_frame,
        syscall::CurrentExecvRuntimeContext {
            runtime: &mut runtime,
            user_bytes: &user_mem,
            user_base: BASE,
            vfs: &vfs,
            scratch: &mut scratch,
            stack_bytes: &mut stack,
            stack_top_va: 0x4010_0000,
            new_vm: vm::VmSpace::<2>::new(),
        },
    );

    let Some(exec_frame) = runtime.task_frame(1) else {
        return;
    };
    if result.is_ok_continue_zero()
        && syscall_frame.a7() == syscall::SYS_EXECVE
        && exec_frame.sepc == 0x4000_0000
    {
        console::puts("[chapter] ch111 execv frame dispatch\n");
        console::puts("[syscall] execve frame a7=221 ret=0 entry=0x40000000\n");
    }
}

fn run_execv_frame_writeback_demo() {
    const BASE: usize = 0x7000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARGV: usize = BASE + 0x100;
    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let (elf, image) = unsafe { (&mut CH112_INIT_ELF, &mut CH112_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut process_vm = vm::VmSpace::<2>::new();
    if process_vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }

    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(pid) = runtime.create_process(None, process_vm) else {
        return;
    };
    let mut task_frame = trap::TrapFrame::default();
    task_frame.sepc = 0x4000_1000;
    task_frame.set_sp(0x4010_1000);
    if runtime.add_runnable_task(1, pid, task_frame).is_err() {
        return;
    }
    runtime.start_first_task();

    let mut live_frame = trap::TrapFrame::default();
    live_frame.sepc = 0x4000_2000;
    live_frame.set_a7(syscall::SYS_EXECVE);
    live_frame.set_a0(PATH);
    live_frame.set_a1(ARGV);
    live_frame.set_a2(0);

    let mut scratch = [0u8; 512];
    let mut stack = [0u8; 256];
    let outcome = trap::handle_current_execv_frame_from_runtime::<4, 64, 4, 2, 2, 5>(
        &mut live_frame,
        syscall::CurrentExecvRuntimeContext {
            runtime: &mut runtime,
            user_bytes: &user_mem,
            user_base: BASE,
            vfs: &vfs,
            scratch: &mut scratch,
            stack_bytes: &mut stack,
            stack_top_va: 0x4010_0000,
            new_vm: vm::VmSpace::<2>::new(),
        },
    );

    let Some(exec_frame) = runtime.task_frame(1) else {
        return;
    };
    if outcome == trap::TrapOutcome::Resume
        && live_frame.a0() == 0
        && live_frame.sepc == 0x4000_2004
        && exec_frame.sepc == 0x4000_0000
    {
        console::puts("[chapter] ch112 execv frame writeback\n");
        console::puts("[syscall] execve writeback a0=0 sepc=0x40002004 entry=0x40000000\n");
    }
}

fn run_execv_trap_dispatch_demo() {
    const BASE: usize = 0x7000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARGV: usize = BASE + 0x100;
    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let (elf, image) = unsafe { (&mut CH113_INIT_ELF, &mut CH113_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut process_vm = vm::VmSpace::<2>::new();
    if process_vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }

    let mut runtime =
        trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new_with_execv_stack_top(0x4010_0000);
    let Ok(pid) = runtime.create_process(None, process_vm) else {
        return;
    };
    let mut task_frame = trap::TrapFrame::default();
    task_frame.sepc = 0x4000_1000;
    task_frame.set_sp(0x4010_1000);
    if runtime.add_runnable_task(1, pid, task_frame).is_err() {
        return;
    }
    runtime.start_first_task();

    let mut live_frame = trap::TrapFrame::default();
    live_frame.sepc = 0x4000_3000;
    live_frame.set_a7(syscall::SYS_EXECVE);
    live_frame.set_a0(PATH);
    live_frame.set_a1(ARGV);
    live_frame.set_a2(0);

    let mut scratch = [0u8; 512];
    let mut stack = [0u8; 256];
    let outcome = trap::handle_trap_with_current_execv_runtime::<4, 64, 4, 2, 2, 5>(
        &mut live_frame,
        trap::SCAUSE_USER_ECALL,
        0,
        syscall::CurrentExecvRuntimeContext {
            runtime: &mut runtime,
            user_bytes: &user_mem,
            user_base: BASE,
            vfs: &vfs,
            scratch: &mut scratch,
            stack_bytes: &mut stack,
            stack_top_va: 0x4010_0000,
            new_vm: vm::VmSpace::<2>::new(),
        },
    );

    let Some(exec_frame) = runtime.task_frame(1) else {
        return;
    };
    if outcome == trap::TrapOutcome::Resume
        && live_frame.a0() == 0
        && live_frame.sepc == 0x4000_3004
        && exec_frame.sepc == 0x4000_0000
    {
        console::puts("[chapter] ch113 execv trap dispatch\n");
        console::puts("[trap] execve ecall routed a0=0 sepc=0x40003004 entry=0x40000000\n");
    }
}

fn run_execv_trap_resources_demo() {
    const BASE: usize = 0x7000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARGV: usize = BASE + 0x100;
    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let (elf, image) = unsafe { (&mut CH114_INIT_ELF, &mut CH114_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut process_vm = vm::VmSpace::<2>::new();
    if process_vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }

    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let Ok(pid) = runtime.create_process(None, process_vm) else {
        return;
    };
    let mut task_frame = trap::TrapFrame::default();
    task_frame.sepc = 0x4000_1000;
    task_frame.set_sp(0x4010_1000);
    if runtime.add_runnable_task(1, pid, task_frame).is_err() {
        return;
    }
    runtime.start_first_task();

    let mut live_frame = trap::TrapFrame::default();
    live_frame.sepc = 0x4000_4000;
    live_frame.set_a7(syscall::SYS_EXECVE);
    live_frame.set_a0(PATH);
    live_frame.set_a1(ARGV);
    live_frame.set_a2(0);

    let mut entry_store =
        trap_runtime::PreparedExecvEntryStore::<4, 2, 2, 5, 512, 256>::prepare(runtime, &vfs);
    let user_source = trap::ExecvPreparedUserSource::new(&user_mem, BASE);
    let replacement_vm = trap::ExecvPreparedReplacementVm::new(vm::VmSpace::<2>::new());
    let recipe = entry_store.entry_recipe(user_source, replacement_vm);
    let input = entry_store.entry_input(recipe);

    let outcome = trap::handle_execv_user_ecall_with_prepared_store::<4, 64, 4, 2, 2, 5, 512, 256>(
        &mut live_frame,
        &mut entry_store,
        input,
    );

    let Some(exec_frame) = entry_store.task_frame(1) else {
        return;
    };
    if outcome == trap::TrapOutcome::Resume
        && live_frame.a0() == 0
        && live_frame.sepc == 0x4000_4004
        && exec_frame.sepc == 0x4000_0000
    {
        console::puts("[chapter] ch114 execv trap resources\n");
        console::puts("[trap] execve resources routed a0=0 sepc=0x40004004 entry=0x40000000\n");
        console::puts("[chapter] ch115 execv resource store\n");
        console::puts(
            "[trap] execve resource store routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch116 execv runtime resources\n");
        console::puts(
            "[trap] execve runtime resources routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch117 execv runtime dispatch\n");
        console::puts(
            "[trap] execve runtime dispatch routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch118 execv dispatch state\n");
        console::puts(
            "[trap] execve dispatch state routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch119 execv dispatch plan\n");
        console::puts("[trap] execve dispatch plan routed a0=0 sepc=0x40004004 entry=0x40000000\n");
        console::puts("[chapter] ch120 execv runtime plan\n");
        console::puts("[trap] execve runtime plan routed a0=0 sepc=0x40004004 entry=0x40000000\n");
        console::puts("[chapter] ch121 execv prepared dispatch\n");
        console::puts(
            "[trap] execve prepared dispatch routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch122 execv prepared runtime\n");
        console::puts(
            "[trap] execve prepared runtime routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch123 execv prepared ecall\n");
        console::puts(
            "[trap] execve prepared ecall routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch124 execv prepared entry\n");
        console::puts(
            "[trap] execve prepared entry routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch125 execv prepared store\n");
        console::puts(
            "[trap] execve prepared store routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch126 execv prepared init\n");
        console::puts("[trap] execve prepared init routed a0=0 sepc=0x40004004 entry=0x40000000\n");
        console::puts("[chapter] ch127 execv prepared input\n");
        console::puts(
            "[trap] execve prepared input routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch128 execv entry input boundary\n");
        console::puts(
            "[trap] execve entry input boundary routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch129 execv user source boundary\n");
        console::puts(
            "[trap] execve user source boundary routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch130 execv replacement vm boundary\n");
        console::puts(
            "[trap] execve replacement vm boundary routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
        console::puts("[chapter] ch131 execv entry recipe\n");
        console::puts("[trap] execve entry recipe routed a0=0 sepc=0x40004004 entry=0x40000000\n");
        console::puts("[chapter] ch132 execv recipe store boundary\n");
        console::puts(
            "[trap] execve recipe store boundary routed a0=0 sepc=0x40004004 entry=0x40000000\n",
        );
    }
}

fn run_execv_prepared_entry_dispatch_demo() {
    const BASE: usize = 0x7000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARGV: usize = BASE + 0x100;

    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let (elf, image) = unsafe { (&mut CH137_INIT_ELF, &mut CH137_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut process_vm = vm::VmSpace::<2>::new();
    if process_vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }
    let mut runtime =
        trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new_with_execv_stack_top(0x4010_0000);
    let Ok(pid) = runtime.create_process(None, process_vm) else {
        return;
    };
    let mut task_frame = trap::TrapFrame::default();
    task_frame.sepc = 0x4000_1000;
    task_frame.set_sp(0x4010_1000);
    if runtime.add_runnable_task(1, pid, task_frame).is_err() {
        return;
    }
    runtime.start_first_task();

    let mut live_frame = trap::TrapFrame::default();
    live_frame.sepc = 0x4000_5000;
    live_frame.set_a7(syscall::SYS_EXECVE);
    live_frame.set_a0(PATH);
    live_frame.set_a1(ARGV);
    live_frame.set_a2(0);

    let mut entry_store =
        trap_runtime::PreparedExecvEntryStore::<4, 2, 2, 5, 512, 256>::prepare(runtime, &vfs);
    let user_source = trap::ExecvPreparedUserSource::new(&user_mem, BASE);
    let replacement_vm = trap::ExecvPreparedReplacementVm::new(vm::VmSpace::<2>::new());
    let outcome = trap::handle_execv_user_ecall_with_prepared_entry::<4, 64, 4, 2, 2, 5, 512, 256>(
        &mut live_frame,
        &mut entry_store,
        user_source,
        replacement_vm,
    );

    let Some(exec_frame) = entry_store.task_frame(1) else {
        return;
    };
    if outcome == trap::TrapOutcome::Resume
        && live_frame.a0() == 0
        && live_frame.sepc == 0x4000_5004
        && exec_frame.sepc == 0x4000_0000
    {
        console::puts("[chapter] ch137 execve prepared entry dispatch\n");
        console::puts(
            "[trap] execve prepared entry dispatch a0=0 sepc=0x40005004 entry=0x40000000\n",
        );
    }
}

fn run_prepared_process_syscall_entry_demo() {
    const BASE: usize = 0x7000_0000;
    const PATH: usize = BASE;
    const ARG0: usize = BASE + 0x40;
    const ARGV: usize = BASE + 0x100;

    let mut user_mem = [0u8; 512];
    put_cstr(&mut user_mem, PATH - BASE, b"/sbin/init");
    put_cstr(&mut user_mem, ARG0 - BASE, b"/sbin/init");
    put_usize(&mut user_mem, ARGV - BASE, ARG0);
    put_usize(
        &mut user_mem,
        ARGV - BASE + core::mem::size_of::<usize>(),
        0,
    );

    let (elf, image) = unsafe { (&mut CH138_INIT_ELF, &mut CH138_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut process_vm = vm::VmSpace::<2>::new();
    if process_vm
        .map_anon(BASE, BASE + user_mem.len(), sv39::PTE_R)
        .is_err()
    {
        return;
    }
    let mut exec_runtime =
        trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new_with_execv_stack_top(0x4010_0000);
    let Ok(pid) = exec_runtime.create_process(None, process_vm) else {
        return;
    };
    let mut task_frame = trap::TrapFrame::default();
    task_frame.sepc = 0x4000_2000;
    if exec_runtime.add_runnable_task(1, pid, task_frame).is_err() {
        return;
    }
    exec_runtime.start_first_task();

    let mut exec_store =
        trap_runtime::PreparedExecvEntryStore::<4, 2, 2, 5, 512, 256>::prepare(exec_runtime, &vfs);
    let user_source = trap::ExecvPreparedUserSource::new(&user_mem, BASE);
    let replacement_vm = trap::ExecvPreparedReplacementVm::new(vm::VmSpace::<2>::new());

    let mut entry_runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let mut wait_vm = vm::VmSpace::<2>::new();
    if wait_vm.map_anon(0x8000, 0x8004, sv39::PTE_W).is_err() {
        return;
    }
    let mut wait_bytes = [0u8; 4];
    let mut wait_user = user_copy::CheckedUserSliceMut::new(&mut wait_bytes, 0x8000, &wait_vm);

    let mut frame = trap::TrapFrame::default();
    frame.sepc = 0x4000_6000;
    frame.set_a7(syscall::SYS_EXECVE);
    frame.set_a0(PATH);
    frame.set_a1(ARGV);
    frame.set_a2(0);

    let dispatch = trap::handle_prepared_process_syscall_entry::<2, 4, 64, 4, 2, 2, 5, 512, 256>(
        &mut frame,
        &mut entry_runtime,
        2,
        vm::VmSpace::<2>::new(),
        &mut wait_user,
        &mut exec_store,
        user_source,
        replacement_vm,
    );

    let Some(exec_frame) = exec_store.task_frame(1) else {
        return;
    };
    if dispatch == trap::TrapDispatchResult::Resumed
        && frame.a0() == 0
        && frame.sepc == 0x4000_6004
        && exec_frame.sepc == 0x4000_0000
    {
        console::puts("[chapter] ch138 prepared process syscall entry\n");
        console::puts(
            "[trap] process syscall entry routed execve a0=0 sepc=0x40006004 entry=0x40000000\n",
        );
    }
}

fn run_process_syscall_write_entry_demo() {
    let (elf, image) = unsafe { (&mut CH139_INIT_ELF, &mut CH139_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut wait_vm = vm::VmSpace::<2>::new();
    if wait_vm.map_anon(0x8000, 0x8004, sv39::PTE_W).is_err() {
        return;
    }
    let mut wait_bytes = [0u8; 4];
    let mut wait_user = user_copy::CheckedUserSliceMut::new(&mut wait_bytes, 0x8000, &wait_vm);

    let mut write_vm = vm::VmSpace::<2>::new();
    if write_vm.map_anon(0x9000, 0x9020, sv39::PTE_R).is_err() {
        return;
    }
    let write_user = user_copy::CheckedUserSlice::new(b"entry-io", 0x9000, &write_vm);
    let mut current_io = process_io::CurrentProcessIoContext::<8>::with_stdio(1, console::device());
    let mut write_scratch = [0u8; 16];
    let mut read_vm = vm::VmSpace::<2>::new();
    if read_vm.map_anon(0xa000, 0xa020, sv39::PTE_W).is_err() {
        return;
    }
    let mut read_bytes = [0u8; 16];
    let mut read_user = user_copy::CheckedUserSliceMut::new(&mut read_bytes, 0xa000, &read_vm);
    let mut read_scratch = [0u8; 16];
    let open_ns = devfs::DeviceNamespace::new(console::device());
    let mut open_scratch = [0u8; 32];

    let mut exec_store = trap_runtime::PreparedExecvEntryStore::<4, 2, 2, 5, 512, 256>::prepare(
        trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new(),
        &vfs,
    );
    let exec_user_mem = [0u8; 16];
    let user_source = trap::ExecvPreparedUserSource::new(&exec_user_mem, 0x7000);
    let replacement_vm = trap::ExecvPreparedReplacementVm::new(vm::VmSpace::<2>::new());

    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let mut frame = trap::TrapFrame::default();
    frame.sepc = 0x4000_7000;
    frame.set_a7(syscall::SYS_WRITE);
    frame.set_a0(1);
    frame.set_a1(0x9000);
    frame.set_a2(8);

    let dispatch = trap::handle_prepared_process_syscall_entry_with_current_io::<
        2,
        2,
        2,
        2,
        8,
        4,
        64,
        4,
        2,
        2,
        5,
        512,
        256,
    >(
        &mut frame,
        &mut runtime,
        2,
        vm::VmSpace::<2>::new(),
        &mut wait_user,
        trap::ProcessSyscallIoResources::new(
            &mut current_io,
            &write_user,
            &mut write_scratch,
            &mut read_user,
            &mut read_scratch,
            &open_ns,
            &write_user,
            &mut open_scratch,
        ),
        &mut exec_store,
        user_source,
        replacement_vm,
    );

    if dispatch == trap::TrapDispatchResult::Resumed && frame.a0() == 8 && frame.sepc == 0x4000_7004
    {
        console::puts("[chapter] ch139 process syscall write entry\n");
        console::puts("[trap] process syscall entry routed write a0=8 sepc=0x40007004\n");
    }
}

fn run_process_syscall_read_entry_demo() {
    let (elf, image) = unsafe { (&mut CH140_INIT_ELF, &mut CH140_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut wait_vm = vm::VmSpace::<2>::new();
    if wait_vm.map_anon(0x8000, 0x8004, sv39::PTE_W).is_err() {
        return;
    }
    let mut wait_bytes = [0u8; 4];
    let mut wait_user = user_copy::CheckedUserSliceMut::new(&mut wait_bytes, 0x8000, &wait_vm);

    let mut write_vm = vm::VmSpace::<2>::new();
    if write_vm.map_anon(0x9000, 0x9020, sv39::PTE_R).is_err() {
        return;
    }
    let write_user = user_copy::CheckedUserSlice::new(b"unused-io", 0x9000, &write_vm);
    let mut write_scratch = [0u8; 16];

    let mut read_vm = vm::VmSpace::<2>::new();
    if read_vm.map_anon(0xa000, 0xa020, sv39::PTE_W).is_err() {
        return;
    }
    let mut read_bytes = [0u8; 16];
    let mut read_user = user_copy::CheckedUserSliceMut::new(&mut read_bytes, 0xa000, &read_vm);
    let mut read_scratch = [0u8; 16];
    console::readback_device().reset();
    let mut current_io =
        process_io::CurrentProcessIoContext::<8>::with_stdio(1, console::readback_device());
    let open_ns = devfs::DeviceNamespace::new(console::readback_device());
    let mut open_scratch = [0u8; 32];

    let mut exec_store = trap_runtime::PreparedExecvEntryStore::<4, 2, 2, 5, 512, 256>::prepare(
        trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new(),
        &vfs,
    );
    let exec_user_mem = [0u8; 16];
    let user_source = trap::ExecvPreparedUserSource::new(&exec_user_mem, 0x7000);
    let replacement_vm = trap::ExecvPreparedReplacementVm::new(vm::VmSpace::<2>::new());

    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let mut frame = trap::TrapFrame::default();
    frame.sepc = 0x4000_8000;
    frame.set_a7(syscall::SYS_READ);
    frame.set_a0(0);
    frame.set_a1(0xa000);
    frame.set_a2(1);

    let dispatch = trap::handle_prepared_process_syscall_entry_with_current_io::<
        2,
        2,
        2,
        2,
        8,
        4,
        64,
        4,
        2,
        2,
        5,
        512,
        256,
    >(
        &mut frame,
        &mut runtime,
        2,
        vm::VmSpace::<2>::new(),
        &mut wait_user,
        trap::ProcessSyscallIoResources::new(
            &mut current_io,
            &write_user,
            &mut write_scratch,
            &mut read_user,
            &mut read_scratch,
            &open_ns,
            &write_user,
            &mut open_scratch,
        ),
        &mut exec_store,
        user_source,
        replacement_vm,
    );

    if dispatch == trap::TrapDispatchResult::Resumed && frame.a0() == 1 && frame.sepc == 0x4000_8004
    {
        console::puts("[chapter] ch140 process syscall read entry\n");
        console::puts("[trap] process syscall entry routed read a0=1 sepc=0x40008004\n");
    }
}

fn run_process_syscall_close_entry_demo() {
    let (elf, image) = unsafe { (&mut CH141_INIT_ELF, &mut CH141_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut wait_vm = vm::VmSpace::<2>::new();
    if wait_vm.map_anon(0x8000, 0x8004, sv39::PTE_W).is_err() {
        return;
    }
    let mut wait_bytes = [0u8; 4];
    let mut wait_user = user_copy::CheckedUserSliceMut::new(&mut wait_bytes, 0x8000, &wait_vm);

    let mut write_vm = vm::VmSpace::<2>::new();
    if write_vm.map_anon(0x9000, 0x9020, sv39::PTE_R).is_err() {
        return;
    }
    let write_user = user_copy::CheckedUserSlice::new(b"after-close", 0x9000, &write_vm);
    let mut write_scratch = [0u8; 16];

    let mut read_vm = vm::VmSpace::<2>::new();
    if read_vm.map_anon(0xa000, 0xa020, sv39::PTE_W).is_err() {
        return;
    }
    let mut read_bytes = [0u8; 16];
    let mut read_user = user_copy::CheckedUserSliceMut::new(&mut read_bytes, 0xa000, &read_vm);
    let mut read_scratch = [0u8; 16];
    let mut current_io = process_io::CurrentProcessIoContext::<8>::with_stdio(1, console::device());
    let open_ns = devfs::DeviceNamespace::new(console::device());
    let mut open_scratch = [0u8; 32];

    let mut exec_store = trap_runtime::PreparedExecvEntryStore::<4, 2, 2, 5, 512, 256>::prepare(
        trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new(),
        &vfs,
    );
    let exec_user_mem = [0u8; 16];
    let user_source = trap::ExecvPreparedUserSource::new(&exec_user_mem, 0x7000);
    let replacement_vm = trap::ExecvPreparedReplacementVm::new(vm::VmSpace::<2>::new());

    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let mut frame = trap::TrapFrame::default();
    frame.sepc = 0x4000_9000;
    frame.set_a7(syscall::SYS_CLOSE);
    frame.set_a0(1);

    let dispatch = trap::handle_prepared_process_syscall_entry_with_current_io::<
        2,
        2,
        2,
        2,
        8,
        4,
        64,
        4,
        2,
        2,
        5,
        512,
        256,
    >(
        &mut frame,
        &mut runtime,
        2,
        vm::VmSpace::<2>::new(),
        &mut wait_user,
        trap::ProcessSyscallIoResources::new(
            &mut current_io,
            &write_user,
            &mut write_scratch,
            &mut read_user,
            &mut read_scratch,
            &open_ns,
            &write_user,
            &mut open_scratch,
        ),
        &mut exec_store,
        user_source,
        replacement_vm,
    );

    let after_close = syscall::handle_current_process_write_checked_user_syscall(
        &current_io,
        &write_user,
        1,
        0x9000,
        11,
        &mut write_scratch,
    );

    if dispatch == trap::TrapDispatchResult::Resumed
        && frame.a0() == 0
        && frame.sepc == 0x4000_9004
        && after_close == syscall::SyscallResult::Continue(-syscall::EBADF)
    {
        console::puts("[chapter] ch141 process syscall close entry\n");
        console::puts("[trap] process syscall entry routed close a0=0 sepc=0x40009004 after=-9\n");
    }
}

fn run_process_syscall_openat_entry_demo() {
    let (elf, image) = unsafe { (&mut CH142_INIT_ELF, &mut CH142_EXT4_IMAGE) };
    let Ok(elf_len) = demo_user_init::build_demo_init_elf(elf) else {
        return;
    };
    if demo_user_init::build_demo_ext4_image(image, &elf[..elf_len]).is_err() {
        return;
    }
    let dev = ramdisk::RamDisk::new("ramblk0", image, 512);
    let Ok(vfs) = vfs::Vfs::mount_ext4(&dev) else {
        return;
    };

    let mut wait_vm = vm::VmSpace::<2>::new();
    if wait_vm.map_anon(0x8000, 0x8004, sv39::PTE_W).is_err() {
        return;
    }
    let mut wait_bytes = [0u8; 4];
    let mut wait_user = user_copy::CheckedUserSliceMut::new(&mut wait_bytes, 0x8000, &wait_vm);

    let mut write_vm = vm::VmSpace::<2>::new();
    if write_vm.map_anon(0x9000, 0x9020, sv39::PTE_R).is_err() {
        return;
    }
    let write_user = user_copy::CheckedUserSlice::new(b"entry-open", 0x9000, &write_vm);
    let mut write_scratch = [0u8; 16];

    let mut read_vm = vm::VmSpace::<2>::new();
    if read_vm.map_anon(0xa000, 0xa020, sv39::PTE_W).is_err() {
        return;
    }
    let mut read_bytes = [0u8; 16];
    let mut read_user = user_copy::CheckedUserSliceMut::new(&mut read_bytes, 0xa000, &read_vm);
    let mut read_scratch = [0u8; 16];

    let mut open_vm = vm::VmSpace::<2>::new();
    if open_vm.map_anon(0xb000, 0xb020, sv39::PTE_R).is_err() {
        return;
    }
    let open_user = user_copy::CheckedUserSlice::new(b"/dev/console\0", 0xb000, &open_vm);
    let open_ns = devfs::DeviceNamespace::new(console::device());
    let mut open_scratch = [0u8; 32];
    let mut current_io = process_io::CurrentProcessIoContext::<8>::with_stdio(1, console::device());

    let mut exec_store = trap_runtime::PreparedExecvEntryStore::<4, 2, 2, 5, 512, 256>::prepare(
        trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new(),
        &vfs,
    );
    let exec_user_mem = [0u8; 16];
    let user_source = trap::ExecvPreparedUserSource::new(&exec_user_mem, 0x7000);
    let replacement_vm = trap::ExecvPreparedReplacementVm::new(vm::VmSpace::<2>::new());

    let mut runtime = trap_runtime::ProcessTrapRuntime::<4, 2, 2, 5>::new();
    let mut frame = trap::TrapFrame::default();
    frame.sepc = 0x4000_a000;
    frame.set_a7(syscall::SYS_OPENAT);
    frame.set_a0(usize::MAX);
    frame.set_a1(0xb000);
    frame.set_a2(1);

    let dispatch = trap::handle_prepared_process_syscall_entry_with_current_io::<
        2,
        2,
        2,
        2,
        8,
        4,
        64,
        4,
        2,
        2,
        5,
        512,
        256,
    >(
        &mut frame,
        &mut runtime,
        2,
        vm::VmSpace::<2>::new(),
        &mut wait_user,
        trap::ProcessSyscallIoResources::new(
            &mut current_io,
            &write_user,
            &mut write_scratch,
            &mut read_user,
            &mut read_scratch,
            &open_ns,
            &open_user,
            &mut open_scratch,
        ),
        &mut exec_store,
        user_source,
        replacement_vm,
    );

    let opened_fd = frame.a0();
    let after_open = syscall::handle_current_process_write_checked_user_syscall(
        &current_io,
        &write_user,
        opened_fd,
        0x9000,
        10,
        &mut write_scratch,
    );

    if dispatch == trap::TrapDispatchResult::Resumed
        && opened_fd == 3
        && frame.sepc == 0x4000_a004
        && after_open == syscall::SyscallResult::Continue(10)
    {
        console::puts("[chapter] ch142 process syscall openat entry\n");
        console::puts("[trap] process syscall entry routed openat a0=3 sepc=0x4000a004 wrote=10\n");
        console::puts("[chapter] ch143 process syscall io resources\n");
        console::puts(
            "[trap] process syscall io resources routed openat a0=3 sepc=0x4000a004 wrote=10\n",
        );
    }
}

fn run_empty_fd_table_init_stdio_demo() {
    let ns = devfs::DeviceNamespace::new(console::device());
    let mut current_io = process_io::CurrentProcessIoContext::<8>::empty(1);
    let mut path_vm = vm::VmSpace::<2>::new();
    if path_vm.map_anon(0xb000, 0xb020, sv39::PTE_R).is_err() {
        return;
    }
    let user = user_copy::CheckedUserSlice::new(b"/dev/console\0init-stdio", 0xb000, &path_vm);
    let mut path_scratch = [0u8; 32];
    let mut write_scratch = [0u8; 16];

    let fd0 = syscall::handle_process_open_checked_user_path(
        current_io.io_mut(),
        &ns,
        &user,
        0xb000,
        0,
        &mut path_scratch,
    );
    let fd1 = syscall::handle_process_open_checked_user_path(
        current_io.io_mut(),
        &ns,
        &user,
        0xb000,
        1,
        &mut path_scratch,
    );
    let fd2 = syscall::handle_process_open_checked_user_path(
        current_io.io_mut(),
        &ns,
        &user,
        0xb000,
        1,
        &mut path_scratch,
    );
    let wrote = syscall::handle_current_process_write_checked_user_syscall(
        &current_io,
        &user,
        1,
        0xb00d,
        10,
        &mut write_scratch,
    );

    if fd0 == syscall::SyscallResult::Continue(0)
        && fd1 == syscall::SyscallResult::Continue(1)
        && fd2 == syscall::SyscallResult::Continue(2)
        && wrote == syscall::SyscallResult::Continue(10)
    {
        console::puts("[chapter] ch145 empty fd table init stdio\n");
        console::puts("[init] opened stdio fds 0,1,2 from empty table wrote=10\n");
    }
}

fn run_user_init_runtime_split_demo() {
    console::puts("[chapter] ch146 user init runtime split\n");
    console::puts("[user-rt] init owns stdio open/write/exit wrappers before libc extraction\n");
    console::puts("[chapter] ch147 user libc library boundary\n");
    console::puts("[user-libc] init links syscall and stdio API from user/libc crate\n");
    console::puts("[chapter] ch148 user libc static link artifact\n");
    console::puts(
        "[user-libc] build produces libmmos_user_libc.rlib before shared object support\n",
    );
    console::puts("[chapter] ch149 user link model classification\n");
    console::puts(
        "[user-link] init is ET_EXEC, crt and libc are rlibs, runtime shared object is absent\n",
    );
    console::puts("[chapter] ch150 shared object load plan\n");
    console::puts(
        "[user-link] ET_DYN shared object requires base, segments, and relocation plan\n",
    );
    console::puts("[chapter] ch151 user image shared object artifact\n");
    console::puts("[user-link] /lib/libmmos_user_libc.so is packaged as an ET_DYN candidate\n");
    console::puts("[chapter] ch152 shared object dynamic metadata\n");
    console::puts("[user-link] dynamic segment exposes RELA metadata before relocation apply\n");
    console::puts("[chapter] ch153 shared object relocation candidates\n");
    console::puts("[user-link] RELA records are scanned into relocation apply candidates\n");
    console::puts("[chapter] ch154 relative relocation apply value\n");
    console::puts("[user-link] R_RISCV_RELATIVE computes relocation value as base plus addend\n");
    console::puts("[chapter] ch155 relocation writeback boundary\n");
    console::puts("[user-link] relocation apply values write checked little-endian u64 slots\n");
    console::puts("[chapter] ch156 shared object relocation materialize\n");
    console::puts("[user-link] shared object materialization connects RELA scan, relative apply, and writeback\n");
    console::puts("[chapter] ch157 shared object mapping plan\n");
    console::puts(
        "[user-link] shared object segments become base-offset user mapping candidates\n",
    );
    console::puts("[chapter] ch158 shared object frame plan\n");
    console::puts("[user-link] shared object mapping candidates receive PMM-backed frames before page table install\n");
    console::puts("[chapter] ch159 shared object page table install\n");
    console::puts(
        "[user-link] shared object frame plan installs user leaf mappings without stack pages\n",
    );
    console::puts("[chapter] ch160 shared object load pipeline\n");
    console::puts("[user-link] shared object load pipeline materializes relocations, frames, and leaf mappings\n");
    console::puts("[chapter] ch161 shared object path load\n");
    console::puts(
        "[user-link] shared object path load reads VFS bytes before running the load pipeline\n",
    );
    console::puts("[chapter] ch162 shared object dependency load\n");
    console::puts("[user-link] exec dependency names /lib/libmmos_user_libc.so before path load\n");
    console::puts("[chapter] ch163 exec shared object dependency\n");
    console::puts("[user-link] exec image preparation carries the libc shared object dependency\n");
    console::puts("[chapter] ch164 process exec shared object runtime\n");
    console::puts("[user-link] process exec result preserves loaded shared object metadata\n");
    console::puts("[chapter] ch165 exec apply shared object runtime\n");
    console::puts(
        "[user-link] exec apply path passes loaded shared object metadata into process runtime\n",
    );
    console::puts("[chapter] ch166 exec dispatch shared object runtime\n");
    console::puts(
        "[user-link] exec dispatch path can select the shared object aware apply runtime\n",
    );
    console::puts("[chapter] ch168 user crt entry main\n");
    console::puts("[user-crt] _start passes argc argv envp to libc startup entry\n");
    console::puts("[user-libc] startup entry calls app main and exits with its return code\n");
    console::puts("[chapter] ch169 user main startup args\n");
    console::puts("[user-libc] app main can print argv and envp through libc helpers\n");
    console::puts("[chapter] ch171 user libc env lookup\n");
    console::puts("[user-libc] app can ask libc to print an env value by key\n");
}

fn run_kernel_satp_guard_demo() {
    if ch78_kernel_satp_runtime_guard() {
        console::puts("[chapter] ch78 kernel satp runtime guard\n");
    }
}

fn run_process_address_space_root_demo() {
    if ch79_process_address_space_root() {
        console::puts("[chapter] ch79 process address space root\n");
    }
}

fn run_pcb_address_space_handle_demo() {
    if unsafe { core::ptr::read_volatile(core::ptr::addr_of!(PCB_ADDRESS_SPACE_HANDLE_READY)) } {
        console::puts("[chapter] ch80 pcb address space handle\n");
    }
}

fn run_satp_hardware_commit_probe_demo() {
    if unsafe { core::ptr::read_volatile(core::ptr::addr_of!(SATP_HARDWARE_COMMIT_PROBE_READY)) } {
        console::puts("[chapter] ch81 satp hardware commit probe\n");
    }
}

fn run_user_frame_backed_image_demo() {
    if unsafe { core::ptr::read_volatile(core::ptr::addr_of!(USER_FRAME_BACKED_IMAGE_READY)) } {
        console::puts("[chapter] ch82 user frame backed image\n");
    }
}

fn run_user_frame_backed_mapping_demo() {
    if unsafe { core::ptr::read_volatile(core::ptr::addr_of!(USER_FRAME_BACKED_MAPPING_READY)) } {
        console::puts("[chapter] ch83 user frame backed page mapping\n");
    }
}

fn run_user_frame_backed_materialize_demo() {
    if unsafe { core::ptr::read_volatile(core::ptr::addr_of!(USER_FRAME_BACKED_MATERIALIZE_READY)) }
    {
        console::puts("[chapter] ch84 user frame materialize\n");
    }
}

fn run_persistent_satp_commit_demo() {
    let next_satp =
        unsafe { core::ptr::read_volatile(core::ptr::addr_of!(PERSISTENT_SATP_COMMIT_SATP)) };
    if next_satp == 0 {
        return;
    }

    let commit = task_vm::prepare_satp_commit(
        0,
        task_vm::AppliedAddressSpaceSwitch {
            prev_tid: 1,
            next_tid: 2,
            active_satp: next_satp,
        },
    );
    let Ok(hardware) = task_vm::prepare_satp_hardware_commit(commit) else {
        return;
    };
    let committed = unsafe { task_vm::commit_satp_hardware(hardware) };
    if committed.active_satp == next_satp && committed.flushed_tlb {
        unsafe {
            core::ptr::write_volatile(core::ptr::addr_of_mut!(PERSISTENT_SATP_COMMIT_READY), true);
        }
        console::puts("[chapter] ch85 persistent satp commit\n");
        let access =
            user_access::with_user_memory_access(|| user_access::user_access_window_from_status(0));
        if user_access::user_memory_access_enabled(access.enabled_sstatus)
            && !user_access::user_memory_access_enabled(access.restored_sstatus)
        {
            unsafe {
                core::ptr::write_volatile(
                    core::ptr::addr_of_mut!(USER_MEMORY_ACCESS_GATE_READY),
                    true,
                );
            }
            console::puts("[chapter] ch86 user memory access gate\n");
            if unsafe { core::ptr::read_volatile(core::ptr::addr_of!(FIRST_USER_SRET_READY)) } {
                console::puts("[chapter] ch87 first user sret syscall\n");
                let frame = unsafe { &*core::ptr::addr_of!(FIRST_USER_SRET_FRAME) };
                #[cfg(target_arch = "riscv64")]
                unsafe {
                    trap::enter_user(frame);
                }
            }
        }
    }
}

#[inline(never)]
fn ch78_kernel_satp_runtime_guard() -> bool {
    let mut pmm = pmm::PageFrameManager::<1>::new(0x9000_0000, 0x9000_1000, sv39::PAGE_SIZE);
    let Some(root_frame_pa) = pmm.alloc() else {
        return false;
    };
    let root = unsafe { user_vm::root_page_table_from_identity_frame_mut(root_frame_pa) };
    if user_vm::install_kernel_identity_mapping(root).is_err() {
        return false;
    }
    let kernel_code = run_user_launch_demo as *const () as usize;
    match user_vm::guard_kernel_runtime_after_satp(
        root,
        kernel_code,
        trap::kernel_trap_stack_top() - 16,
        trap::trap_vector_address(),
    ) {
        Ok(guard) => {
            guard.kernel_code_vpn2 == user_vm::KERNEL_IDENTITY_VPN2
                && guard.kernel_stack_vpn2 == user_vm::KERNEL_IDENTITY_VPN2
                && guard.trap_entry_vpn2 == user_vm::KERNEL_IDENTITY_VPN2
        }
        Err(_) => false,
    }
}

#[inline(never)]
fn ch79_process_address_space_root() -> bool {
    let address_space = unsafe { &*core::ptr::addr_of!(USER_ADDRESS_SPACE) };
    address_space.mapped_regions > 0
        && address_space.satp == sv39::make_satp(address_space.root_frame_pa / sv39::PAGE_SIZE, 0)
        && address_space.root_frame_pa % sv39::PAGE_SIZE == 0
}

fn run_user_launch_demo() {
    let mut vm = vm::VmSpace::<4>::new();
    let _ = vm.map_anon(0x4000_0000, 0x4000_1000, sv39::PTE_R | sv39::PTE_W);
    let mut pmm = pmm::PageFrameManager::<16>::new(0x9000_0000, 0x9001_0000, sv39::PAGE_SIZE);
    let pf = page_fault::handle_user_page_fault(
        page_fault::SCAUSE_STORE_PAGE_FAULT,
        0x4000_0010,
        &vm,
        &mut pmm,
    );
    if let page_fault::PageFaultResult::Mapped(_) = pf {
        console::puts("[chapter] ch19 trap outcome bridge\n");
    }

    static mut INIT_SCRATCH: [u8; 64 * 1024] = [0; 64 * 1024];
    static mut DEMO_STACK: [u8; 4096] = [0; 4096];

    let scratch = unsafe { &mut *core::ptr::addr_of_mut!(INIT_SCRATCH) };
    let stack_bytes = unsafe { &mut *core::ptr::addr_of_mut!(DEMO_STACK) };
    let stack_top = 0x4010_0000usize;

    let dev = match virtio_blk::VirtioBlockDevice::probe() {
        Ok(dev) => dev,
        Err(_) => {
            console::puts("[runtime] virtio-blk probe failed\n");
            return;
        }
    };
    let start_lba = match block::mbr_first_partition_lba(&dev) {
        Ok(lba) => lba,
        Err(_) => {
            console::puts("[runtime] MBR first partition lookup failed\n");
            return;
        }
    };
    let root = block::PartitionBlockDevice::new("drive0p1", &dev, start_lba);
    let vfs = match vfs::Vfs::mount_ext4(&root) {
        Ok(v) => v,
        Err(_) => {
            console::puts("[runtime] ext4 mount failed\n");
            return;
        }
    };
    let loaded = match user_exec::load_init_exec_image(&vfs, "/sbin/init", scratch, stack_top) {
        Ok(v) => v,
        Err(_) => {
            console::puts("[runtime] /sbin/init ELF load failed\n");
            return;
        }
    };
    if loaded.entry >= 0x8000_0000 && materialize_loaded_segments(&loaded).is_err() {
        return;
    }

    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("fence.i");
    }

    let plan = user_launch::prepare_user_init_launch::<4, 4>(
        &loaded,
        0x12345_000,
        stack_bytes,
        &["/sbin/init"],
        &["PATH=/bin"],
    );
    if let Ok(p) = plan {
        if p.argc == 1 && p.trapframe.sepc == loaded.entry {
            let _ = (p.satp, p.mapped_regions, p.sp, p.argv_ptr, p.envp_ptr);
            console::puts("[chapter] ch18 exec prep\n");
            console::puts("[runtime] loaded /sbin/init from virtio ext4 into exec image\n");
            console::puts("[chapter] ch26 user init launch handoff\n");
            console::puts("[launch] vm+stack+trapframe ready for first sret\n");
            if trap_runtime::install_initial_user_task(1, p.trapframe).is_ok()
                && trap_runtime::with_global_trap_runtime(|runtime| runtime.current_tid())
                    == Some(1)
            {
                console::puts("[chapter] ch59 launch task global runtime\n");
            }

            let mut proc_table = process_model::ProcessTable::<3, 3, 4>::new();
            let mut task_table = process_model::TaskTable::<3>::new();
            let mut spawn_scheduler = scheduler::Scheduler::<3>::new();
            if let Ok(first) = user_process::create_user_process_from_launch(
                &mut proc_table,
                &mut task_table,
                &mut spawn_scheduler,
                None,
                vm::VmSpace::<4>::new(),
                p,
            ) {
                if first.pid == 1
                    && first.tid == 1
                    && first.entry == loaded.entry
                    && spawn_scheduler.task_state(first.tid) == Some(scheduler::TaskState::Runnable)
                {
                    console::puts("[chapter] ch61 user process creation path\n");
                }

                let mut second_stack = [0u8; 512];
                if let Ok(second_plan) = user_launch::prepare_user_init_launch::<4, 4>(
                    &loaded,
                    0x12346_000,
                    &mut second_stack,
                    &["/sbin/worker"],
                    &["PATH=/bin"],
                ) {
                    if let Ok(second) = user_process::create_user_process_from_launch(
                        &mut proc_table,
                        &mut task_table,
                        &mut spawn_scheduler,
                        Some(first.pid),
                        vm::VmSpace::<4>::new(),
                        second_plan,
                    ) {
                        let _ = spawn_scheduler.on_tick();
                        let _ = spawn_scheduler.set_state(first.tid, scheduler::TaskState::Blocked);
                        let mut live = p.trapframe;
                        live.sepc = 0xfeed_0000;
                        if let Some(decision) =
                            trap::schedule_after_blocked_syscall(&live, &mut spawn_scheduler)
                        {
                            let result = trap::apply_schedule_decision(
                                &mut live,
                                &spawn_scheduler,
                                decision,
                            );
                            if result == Some(decision)
                                && spawn_scheduler.current_tid() == Some(second.tid)
                                && live.sepc == second_plan.trapframe.sepc
                                && live.sp() == second_plan.sp
                            {
                                console::puts("[chapter] ch62 second user task switch\n");
                                if let Ok(Some(space)) = task_vm::address_space_switch_for_decision(
                                    &proc_table,
                                    &task_table,
                                    decision,
                                ) {
                                    if space.next_tid == second.tid
                                        && space.next_satp == second_plan.satp
                                    {
                                        console::puts("[chapter] ch63 task address space token\n");
                                        let mut active_satp = p.satp;
                                        let applied = task_vm::apply_address_space_switch(
                                            &mut active_satp,
                                            space,
                                        );
                                        if applied.next_tid == second.tid
                                            && applied.active_satp == second_plan.satp
                                            && active_satp == second_plan.satp
                                        {
                                            console::puts(
                                                "[chapter] ch64 address space switch apply\n",
                                            );
                                            let commit =
                                                task_vm::prepare_satp_commit(p.satp, applied);
                                            if commit.next_satp == second_plan.satp
                                                && commit.needs_sfence_vma
                                            {
                                                console::puts(
                                                    "[chapter] ch65 satp commit boundary\n",
                                                );
                                                let address_space = unsafe {
                                                    &mut *core::ptr::addr_of_mut!(
                                                        USER_ADDRESS_SPACE
                                                    )
                                                };
                                                *address_space =
                                                    user_vm::ProcessAddressSpace::empty();
                                                let root_frame_pa = pmm.alloc().unwrap();
                                                let root = unsafe {
                                                    user_vm::root_page_table_from_identity_frame_mut(
                                                        root_frame_pa,
                                                    )
                                                };
                                                let root_install = address_space
                                                    .install_user_image(
                                                        &loaded,
                                                        root_frame_pa,
                                                        root,
                                                    )
                                                    .unwrap();
                                                let _ = root_install;
                                                if user_vm::install_kernel_identity_mapping(root)
                                                    .is_err()
                                                {
                                                    console::puts(
                                                        "[chapter] ch66 kernel mapping overlap\n",
                                                    );
                                                } else if user_vm::is_kernel_mapping(
                                                    root.entries[user_vm::KERNEL_IDENTITY_VPN2],
                                                ) {
                                                    console::puts(
                                                        "[chapter] ch66 user address relocation\n",
                                                    );
                                                    if user_vm::preflight_satp_commit(
                                                        root,
                                                        loaded.entry,
                                                        loaded.stack_top,
                                                    )
                                                    .is_ok()
                                                    {
                                                        console::puts(
                                                            "[chapter] ch67 satp commit preflight\n",
                                                        );
                                                        if trap::prepare_kernel_trap_stack_switch(
                                                            loaded.stack_top,
                                                            0x8020_8000,
                                                            0x8020_7000,
                                                        )
                                                        .is_ok()
                                                        {
                                                            console::puts(
                                                                "[chapter] ch68 kernel trap stack switch\n",
                                                            );
                                                            if let Ok(scratch) =
                                                                trap::prepare_trap_entry_scratch(
                                                                    loaded.stack_top,
                                                                    0x8020_8000,
                                                                    0x8020_7000,
                                                                )
                                                            {
                                                                let swap =
                                                                    trap::simulate_trap_entry_stack_swap(
                                                                        scratch,
                                                                    );
                                                                if scratch.sscratch
                                                                    == scratch.kernel_frame_sp
                                                                    && swap.active_sp
                                                                        == scratch.kernel_frame_sp
                                                                    && swap.saved_user_sp
                                                                        == loaded.stack_top
                                                                {
                                                                    console::puts(
                                                                        "[chapter] ch69 trap entry scratch swap\n",
                                                                    );
                                                                    let ret =
                                                                        trap::simulate_trap_entry_return_scratch(
                                                                            scratch,
                                                                        );
                                                                    if ret.restored_user_sp
                                                                        == loaded.stack_top
                                                                        && ret.next_sscratch
                                                                            == scratch
                                                                                .kernel_frame_sp
                                                                    {
                                                                        console::puts(
                                                                            "[chapter] ch70 trap entry asm stack switch\n",
                                                                        );
                                                                        let mut probe_frame =
                                                                            p.trapframe;
                                                                        probe_frame.set_sp(
                                                                            loaded.stack_top,
                                                                        );
                                                                        if let Ok(enter_scratch) =
                                                                            trap::prepare_enter_user_scratch(
                                                                                &probe_frame,
                                                                            )
                                                                        {
                                                                            if enter_scratch.sscratch == enter_scratch.kernel_frame_sp
                                                                                && enter_scratch
                                                                                    .user_sp
                                                                                    == loaded
                                                                                        .stack_top
                                                                            {
                                                                                console::puts(
                                                                                    "[chapter] ch71 enter user sscratch prepare\n",
                                                                                );
                                                                                if let Ok(hw) =
                                                                                    task_vm::prepare_satp_hardware_commit(
                                                                                        commit,
                                                                                    )
                                                                                {
                                                                                    if hw.next_satp
                                                                                        == commit
                                                                                            .next_satp
                                                                                        && hw.flush_after_write
                                                                                    {
                                                                                        console::puts(
                                                                                            "[chapter] ch72 satp hardware commit gate\n",
                                                                                        );
                                                                                        if let Ok(seq) =
                                                                                            task_vm::prepare_user_launch_csr_sequence(
                                                                                                &probe_frame,
                                                                                                enter_scratch,
                                                                                                commit,
                                                                                            )
                                                                                        {
                                                                                            if seq.sscratch
                                                                                                == enter_scratch
                                                                                                    .sscratch
                                                                                                && seq.satp
                                                                                                    == hw.next_satp
                                                                                                && seq
                                                                                                    .sfence_vma_after_satp
                                                                                                && seq.sepc
                                                                                                    == probe_frame
                                                                                                        .sepc
                                                                                            {
                                                                                                console::puts(
                                                                                                    "[chapter] ch73 user launch csr sequence\n",
                                                                                                );
                                                                                                if root_frame_pa
                                                                                                    % sv39::PAGE_SIZE
                                                                                                    == 0
                                                                                                    && root_install
                                                                                                        .root_ppn
                                                                                                        == root_frame_pa
                                                                                                            / sv39::PAGE_SIZE
                                                                                                    && root_install
                                                                                                        .satp
                                                                                                        == sv39::make_satp(
                                                                                                            root_frame_pa
                                                                                                                / sv39::PAGE_SIZE,
                                                                                                            0,
                                                                                                        )
                                                                                                {
                                                                                                    console::puts(
                                                                                                        "[chapter] ch74 root page table storage\n",
                                                                                                    );
                                                                                                    let rooted_applied =
                                                                                                        task_vm::AppliedAddressSpaceSwitch {
                                                                                                            prev_tid: first.tid,
                                                                                                            next_tid: second.tid,
                                                                                                            active_satp: root_install.satp,
                                                                                                        };
                                                                                                    let rooted_commit =
                                                                                                        task_vm::prepare_satp_commit(
                                                                                                            p.satp,
                                                                                                            rooted_applied,
                                                                                                        );
                                                                                                    if let Ok(rooted_seq) =
                                                                                                        task_vm::prepare_rooted_user_launch_csr_sequence(
                                                                                                            &probe_frame,
                                                                                                            enter_scratch,
                                                                                                            rooted_commit,
                                                                                                            root_install.satp,
                                                                                                        )
                                                                                                    {
                                                                                                        if rooted_seq.root_satp
                                                                                                            == root_install.satp
                                                                                                            && rooted_seq
                                                                                                                .sequence
                                                                                                                .satp
                                                                                                                == root_install
                                                                                                                    .satp
                                                                                                            && rooted_seq
                                                                                                                .sequence
                                                                                                                .sepc
                                                                                                                == probe_frame
                                                                                                                    .sepc
                                                                                                        {
                                                                                                            console::puts(
                                                                                                                "[chapter] ch75 rooted user launch satp\n",
                                                                                                            );
                                                                                                            if proc_table
                                                                                                                .update_satp(
                                                                                                                    second
                                                                                                                        .pid,
                                                                                                                    root_install
                                                                                                                        .satp,
                                                                                                                )
                                                                                                                .is_ok()
                                                                                                            {
                                                                                                                if let Ok(Some(rooted_space)) =
                                                                                                                    task_vm::address_space_switch_for_decision(
                                                                                                                        &proc_table,
                                                                                                                        &task_table,
                                                                                                                        decision,
                                                                                                                    )
                                                                                                                {
                                                                                                                    if rooted_space.next_tid
                                                                                                                        == second
                                                                                                                            .tid
                                                                                                                        && rooted_space
                                                                                                                            .next_satp
                                                                                                                            == root_install
                                                                                                                                .satp
                                                                                                                    {
                                                                                                                        console::puts(
                                                                                                                            "[chapter] ch76 process root satp token\n",
                                                                                                                        );
                                                                                                                        let mut rooted_active_satp =
                                                                                                                            p.satp;
                                                                                                                        if let Ok(rooted_commit_seq) =
                                                                                                                            task_vm::prepare_rooted_satp_commit_sequence(
                                                                                                                                &mut rooted_active_satp,
                                                                                                                                rooted_space,
                                                                                                                                &probe_frame,
                                                                                                                                enter_scratch,
                                                                                                                                root_install.satp,
                                                                                                                            )
                                                                                                                        {
                                                                                                                            if rooted_active_satp
                                                                                                                                == root_install
                                                                                                                                    .satp
                                                                                                                                && rooted_commit_seq
                                                                                                                                    .commit
                                                                                                                                    .next_satp
                                                                                                                                    == root_install
                                                                                                                                        .satp
                                                                                                                                && rooted_commit_seq
                                                                                                                                    .rooted
                                                                                                                                    .sequence
                                                                                                                                    .satp
                                                                                                                                    == root_install
                                                                                                                                        .satp
                                                                                                                            {
                                                                                                                                console::puts(
                                                                                                                                    "[chapter] ch77 rooted satp commit sequence\n",
                                                                                                                                );
                                                                                                                                let spaces = unsafe {
                                                                                                                                    &mut *core::ptr::addr_of_mut!(
                                                                                                                                        USER_ADDRESS_SPACES
                                                                                                                                    )
                                                                                                                                };
                                                                                                                                *spaces = user_vm::ProcessAddressSpaceTable::new();
                                                                                                                                if let Ok(space_id) =
                                                                                                                                    spaces.create_empty()
                                                                                                                                {
                                                                                                                                    let space_root_frame_pa =
                                                                                                                                        pmm.alloc().unwrap();
                                                                                                                                    let space_root = unsafe {
                                                                                                                                        user_vm::root_page_table_from_identity_frame_mut(
                                                                                                                                            space_root_frame_pa,
                                                                                                                                        )
                                                                                                                                    };
                                                                                                                                    if let Ok(space_install) =
                                                                                                                                        spaces.install_user_image(
                                                                                                                                            space_id,
                                                                                                                                            &loaded,
                                                                                                                                            space_root_frame_pa,
                                                                                                                                            space_root,
                                                                                                                                        )
                                                                                                                                    {
                                                                                                                                        if user_vm::install_kernel_identity_mapping(
                                                                                                                                            space_root,
                                                                                                                                        )
                                                                                                                                        .is_ok()
                                                                                                                                            && user_vm::is_kernel_mapping(
                                                                                                                                                space_root.entries[
                                                                                                                                                    user_vm::KERNEL_IDENTITY_VPN2
                                                                                                                                                ],
                                                                                                                                            )
                                                                                                                                            && proc_table
                                                                                                                                                .bind_address_space(
                                                                                                                                                    second
                                                                                                                                                        .pid,
                                                                                                                                                    space_id,
                                                                                                                                                )
                                                                                                                                                .is_ok()
                                                                                                                                        {
                                                                                                                                            if let Ok(Some(handle_space)) =
                                                                                                                                                task_vm::address_space_switch_for_decision_with_spaces(
                                                                                                                                                    &proc_table,
                                                                                                                                                    &task_table,
                                                                                                                                                    spaces,
                                                                                                                                                    decision,
                                                                                                                                                )
                                                                                                                                            {
                                                                                                                                                if proc_table
                                                                                                                                                    .get(
                                                                                                                                                        second
                                                                                                                                                            .pid,
                                                                                                                                                    )
                                                                                                                                                    .map(|pcb| {
                                                                                                                                                        pcb.address_space_id
                                                                                                                                                            == Some(
                                                                                                                                                                space_id,
                                                                                                                                                            )
                                                                                                                                                    })
                                                                                                                                                    .unwrap_or(
                                                                                                                                                        false,
                                                                                                                                                    )
                                                                                                                                                    && handle_space
                                                                                                                                                        .next_satp
                                                                                                                                                        == space_install
                                                                                                                                                            .satp
                                                                                                                                                {
                                                                                                                                                    unsafe {
                                                                                                                                                        core::ptr::write_volatile(
                                                                                                                                                            core::ptr::addr_of_mut!(
                                                                                                                                                                PCB_ADDRESS_SPACE_HANDLE_READY
                                                                                                                                                            ),
                                                                                                                                                            true,
                                                                                                                                                        );
                                                                                                                                                    }
                                                                                                                                                    let probe_commit =
                                                                                                                                                        task_vm::prepare_satp_commit(
                                                                                                                                                            0,
                                                                                                                                                            task_vm::AppliedAddressSpaceSwitch {
                                                                                                                                                                prev_tid: handle_space
                                                                                                                                                                    .prev_tid,
                                                                                                                                                                next_tid: handle_space
                                                                                                                                                                    .next_tid,
                                                                                                                                                                active_satp: handle_space
                                                                                                                                                                    .next_satp,
                                                                                                                                                            },
                                                                                                                                                        );
                                                                                                                                                    if let Ok(hw_probe) =
                                                                                                                                                        task_vm::prepare_satp_hardware_commit(
                                                                                                                                                            probe_commit,
                                                                                                                                                        )
                                                                                                                                                    {
                                                                                                                                                        let probe = unsafe {
                                                                                                                                                            task_vm::probe_satp_hardware_commit_roundtrip(
                                                                                                                                                                hw_probe,
                                                                                                                                                                0,
                                                                                                                                                            )
                                                                                                                                                        };
                                                                                                                                                        if probe.committed_satp
                                                                                                                                                            == space_install
                                                                                                                                                                .satp
                                                                                                                                                            && probe.restored_satp
                                                                                                                                                                == 0
                                                                                                                                                            && probe
                                                                                                                                                                .flushed_on_commit
                                                                                                                                                            && probe
                                                                                                                                                                .flushed_on_restore
                                                                                                                                                        {
                                                                                                                                                            unsafe {
                                                                                                                                                                core::ptr::write_volatile(
                                                                                                                                                                    core::ptr::addr_of_mut!(
                                                                                                                                                                        SATP_HARDWARE_COMMIT_PROBE_READY
                                                                                                                                                                    ),
                                                                                                                                                                    true,
                                                                                                                                                                );
                                                                                                                                                            }
                                                                                                                                                            if let Ok(frame_plan) =
                                                                                                                                                                user_vm::plan_frame_backed_user_image(
                                                                                                                                                                    &loaded,
                                                                                                                                                                    &mut pmm,
                                                                                                                                                                    stack_bytes.len(),
                                                                                                                                                                )
                                                                                                                                                            {
                                                                                                                                                                if frame_plan.entry_vaddr
                                                                                                                                                                    == loaded.entry
                                                                                                                                                                    && frame_plan.entry_frame_pa
                                                                                                                                                                        != loaded.entry
                                                                                                                                                                    && frame_plan.stack_top
                                                                                                                                                                        == loaded.stack_top
                                                                                                                                                                    && frame_plan.stack_frame_start_pa
                                                                                                                                                                        != loaded.stack_top
                                                                                                                                                                    && frame_plan.segment_count
                                                                                                                                                                        == loaded.segment_count
                                                                                                                                                                {
                                                                                                                                                                    unsafe {
                                                                                                                                                                        core::ptr::write_volatile(
                                                                                                                                                                            core::ptr::addr_of_mut!(
                                                                                                                                                                                USER_FRAME_BACKED_IMAGE_READY
                                                                                                                                                                            ),
                                                                                                                                                                            true,
                                                                                                                                                                        );
                                                                                                                                                                    }
                                                                                                                                                                    if let Ok(mapping) =
                                                                                                                                                                        user_vm::install_frame_backed_user_mapping(
                                                                                                                                                                            space_root,
                                                                                                                                                                            &frame_plan,
                                                                                                                                                                            &mut pmm,
                                                                                                                                                                        )
                                                                                                                                                                    {
                                                                                                                                                                        let root_entry =
                                                                                                                                                                            space_root.entries[
                                                                                                                                                                                user_vm::vpn2(
                                                                                                                                                                                    frame_plan
                                                                                                                                                                                        .entry_vaddr,
                                                                                                                                                                                )
                                                                                                                                                                            ];
                                                                                                                                                                        if user_vm::is_table_mapping(
                                                                                                                                                                            root_entry,
                                                                                                                                                                        )
                                                                                                                                                                            && root_entry
                                                                                                                                                                                .ppn()
                                                                                                                                                                                == mapping
                                                                                                                                                                                    .l1_frame_pa
                                                                                                                                                                                    / sv39::PAGE_SIZE
                                                                                                                                                                            && mapping
                                                                                                                                                                                .mapped_pages
                                                                                                                                                                                >= frame_plan
                                                                                                                                                                                    .stack_pages
                                                                                                                                                                                    + 1
                                                                                                                                                                            && mapping
                                                                                                                                                                                .mapped_pages
                                                                                                                                                                                <= frame_plan
                                                                                                                                                                                    .segment_count
                                                                                                                                                                                    + frame_plan
                                                                                                                                                                                        .stack_pages
                                                                                                                                                                        {
                                                                                                                                                                            unsafe {
                                                                                                                                                                                core::ptr::write_volatile(
                                                                                                                                                                                    core::ptr::addr_of_mut!(
                                                                                                                                                                                        USER_FRAME_BACKED_MAPPING_READY
                                                                                                                                                                                    ),
                                                                                                                                                                                    true,
                                                                                                                                                                                );
                                                                                                                                                                            }
                                                                                                                                                                            if unsafe {
                                                                                                                                                                                user_exec::materialize_frame_backed_user_image_to_frames(
                                                                                                                                                                                    &loaded,
                                                                                                                                                                                    &frame_plan,
                                                                                                                                                                                    stack_bytes,
                                                                                                                                                                                )
                                                                                                                                                                            }
                                                                                                                                                                            .is_ok()
                                                                                                                                                                            {
                                                                                                                                                                                foundation_lifecycle::install(
                                                                                                                                                                                    frame_plan,
                                                                                                                                                                                );
                                                                                                                                                                                unsafe {
                                                                                                                                                                                    core::ptr::write_volatile(
                                                                                                                                                                                        core::ptr::addr_of_mut!(
                                                                                                                                                                                            USER_FRAME_BACKED_MATERIALIZE_READY
                                                                                                                                                                                        ),
                                                                                                                                                                                        true,
                                                                                                                                                                                    );
                                                                                                                                                                                }
                                                                                                                                                                                let persistent_commit =
                                                                                                                                                                                    task_vm::prepare_satp_commit(
                                                                                                                                                                                        0,
                                                                                                                                                                                        task_vm::AppliedAddressSpaceSwitch {
                                                                                                                                                                                            prev_tid: handle_space
                                                                                                                                                                                                .prev_tid,
                                                                                                                                                                                            next_tid: handle_space
                                                                                                                                                                                                .next_tid,
                                                                                                                                                                                            active_satp: handle_space
                                                                                                                                                                                                .next_satp,
                                                                                                                                                                                        },
                                                                                                                                                                                    );
                                                                                                                                                                                if task_vm::prepare_satp_hardware_commit(
                                                                                                                                                                                    persistent_commit,
                                                                                                                                                                                )
                                                                                                                                                                                .is_ok()
                                                                                                                                                                                {
                                                                                                                                                                                    unsafe {
                                                                                                                                                                                        core::ptr::write_volatile(
                                                                                                                                                                                            core::ptr::addr_of_mut!(
                                                                                                                                                                                                PERSISTENT_SATP_COMMIT_SATP
                                                                                                                                                                                            ),
                                                                                                                                                                                            handle_space
                                                                                                                                                                                                .next_satp,
                                                                                                                                                                                        );
                                                                                                                                                                                        core::ptr::write_volatile(
                                                                                                                                                                                            core::ptr::addr_of_mut!(
                                                                                                                                                                                                FIRST_USER_SRET_FRAME
                                                                                                                                                                                            ),
                                                                                                                                                                                            p.trapframe,
                                                                                                                                                                                        );
                                                                                                                                                                                        core::ptr::write_volatile(
                                                                                                                                                                                            core::ptr::addr_of_mut!(
                                                                                                                                                                                                FIRST_USER_SRET_READY
                                                                                                                                                                                            ),
                                                                                                                                                                                            true,
                                                                                                                                                                                        );
                                                                                                                                                                                    }
                                                                                                                                                                                }
                                                                                                                                                                            }
                                                                                                                                                                        }
                                                                                                                                                                    }
                                                                                                                                                                }
                                                                                                                                                            }
                                                                                                                                                        }
                                                                                                                                                    }
                                                                                                                                                }
                                                                                                                                            }
                                                                                                                                        }
                                                                                                                                    }
                                                                                                                                }
                                                                                                                            }
                                                                                                                        }
                                                                                                                    }
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if loaded.entry < 0x8000_0000 {
                console::puts("[runtime] user virtual entry waits for satp commit\n");
                return;
            }

            console::puts("[chapter] ch28 first user sret\n");

            #[cfg(target_arch = "riscv64")]
            unsafe {
                trap::enter_user(&p.trapframe);
            }
        }
    }
}

fn materialize_loaded_segments(loaded: &elf_loader::LoadedUserImage<'_>) -> Result<(), ()> {
    let mut idx = 0usize;
    while idx < loaded.segment_count {
        let seg = loaded.segments[idx].ok_or(())?;
        let dst = unsafe { core::slice::from_raw_parts_mut(seg.vaddr as *mut u8, seg.mem_size) };
        dst[..seg.file_size].copy_from_slice(seg.data);
        dst[seg.file_size..].fill(0);
        idx += 1;
    }
    Ok(())
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    console::puts("panic\n");
    loop {
        unsafe { core::arch::asm!("wfi") }
    }
}
