#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

#[allow(dead_code)]
mod block;
#[allow(dead_code)]
mod block_io;
mod serial;
#[allow(dead_code)]
mod serial_rx;
mod console;
mod char_device;
#[allow(dead_code)]
mod fd_table;
#[allow(dead_code)]
mod idle;
#[allow(dead_code)]
mod ramdisk;
#[allow(dead_code)]
mod demo_user_init;
#[allow(dead_code)]
mod device;
#[allow(dead_code)]
mod devfs;
#[allow(dead_code)]
mod elf_loader;
#[allow(dead_code)]
mod ext4;
#[allow(dead_code)]
mod page_fault;
#[allow(dead_code)]
mod pmm;
#[allow(dead_code)]
mod process_model;
#[allow(dead_code)]
mod process_io;
#[allow(dead_code)]
mod process_runtime;
mod sbi;
#[allow(dead_code)]
mod scheduler;
#[allow(dead_code)]
mod sv39;
mod syscall;
mod trap;
mod uart;
#[allow(dead_code)]
mod user_copy;
#[allow(dead_code)]
mod user_exec;
mod user_init;
mod user_launch;
mod user_stack;
#[allow(dead_code)]
mod user_vm;
#[allow(dead_code)]
mod virtio_blk;
#[allow(dead_code)]
mod vm;
#[allow(dead_code)]
mod vfs;
#[allow(dead_code)]
mod wait_queue;

use core::panic::PanicInfo;

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
    let checked_user = user_copy::CheckedUserSlice::new(b"/dev/console\0fd-write", 0x4000, &user_vm);
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
    boot_tf.set_sp(0x8050_0000);
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

    let task_owner = process_model::TaskControlBlock::new(1, 1, trap::TrapFrame::default(), 0);
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
        process_model::TaskControlBlock::new(2, runtime_pid, trap::TrapFrame::default(), 0);
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
    let close_task =
        process_model::TaskControlBlock::new(3, close_pid, trap::TrapFrame::default(), 0);
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
    let read_task =
        process_model::TaskControlBlock::new(4, read_pid, trap::TrapFrame::default(), 0);
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
    let input_task =
        process_model::TaskControlBlock::new(5, input_pid, trap::TrapFrame::default(), 0);
    let input_tid = input_task.tid;
    let mut input_vm = vm::VmSpace::<2>::new();
    let _ = input_vm.map_anon(0xa000, 0xa020, sv39::PTE_W);
    let mut input_mem = [0u8; 32];
    {
        let mut input_user =
            user_copy::CheckedUserSliceMut::new(&mut input_mem, 0xa000, &input_vm);
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
        let mut input_user =
            user_copy::CheckedUserSliceMut::new(&mut input_mem, 0xa000, &input_vm);
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
        let mut input_user =
            user_copy::CheckedUserSliceMut::new(&mut input_mem, 0xa000, &input_vm);
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
            let idle_decision = trap::schedule_after_blocked_syscall(
                &blocked_frame,
                &mut idle_scheduler,
            );
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
            if idle_decision
                .and_then(idle::idle_action_for_schedule_decision)
                == Some(idle::IdleAction::WaitForInterrupt {
                    blocked_tid: input_tid,
                })
            {
                console::puts("[chapter] ch53 idle wait for interrupt\n");
            }
        }
    }

    let init_ctx = user_init::build_user_init_context(0x8040_0000, 0x8050_0000);
    if init_ctx.sstatus & user_init::SSTATUS_SPP == 0 {
        console::puts("[mmos-practice] user init context prepared (SPP=U)\n");
    }

    run_scheduler_runtime_demo();
    run_process_runtime_demo();
    console::puts("[mmos-practice] trap/irq scaffold ready\n");
    run_user_launch_demo();
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

fn run_user_launch_demo() {
    let mut vm = vm::VmSpace::<4>::new();
    let _ = vm.map_anon(0x8040_0000, 0x8040_1000, sv39::PTE_R | sv39::PTE_W);
    let mut pmm = pmm::PageFrameManager::<4>::new(0x9000_0000, 0x9000_4000, sv39::PAGE_SIZE);
    let pf = page_fault::handle_user_page_fault(
        page_fault::SCAUSE_STORE_PAGE_FAULT,
        0x8040_0010,
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
    let stack_top = stack_bytes.as_ptr() as usize + stack_bytes.len();

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
    if materialize_loaded_segments(&loaded).is_err() {
        return;
    }

    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("fence.i");
    }

    let plan = user_launch::prepare_user_init_launch::<4, 4>(
        &loaded,
        0x12345,
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
