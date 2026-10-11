// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block::{self, PartitionBlockDevice},
    console,
    trap::{TrapFrame, TrapOutcome},
    user_access, user_exec, user_launch,
    user_vm::UserFrameBackedImagePlan,
    vfs::Vfs,
    virtio_blk::VirtioBlockDevice,
};

const READY: u8 = 1;
const CHILD: u8 = 2;
const ZOMBIE: u8 = 3;
const REAPED: u8 = 4;
const CHILD_PID: usize = 2;
const USER_STACK_BYTES: usize = 4096;
const ELF_SCRATCH_BYTES: usize = 64 * 1024;

static mut STATE: u8 = 0;
static mut CHILD_EXIT_STATUS: i32 = 0;
static mut FRAME_PLAN: Option<UserFrameBackedImagePlan> = None;
static mut PARENT_FRAME: TrapFrame = TrapFrame {
    regs: [0; 32],
    sepc: 0,
    sstatus: 0,
};
static mut SAVED_PARENT_STACK: [u8; USER_STACK_BYTES] = [0; USER_STACK_BYTES];
static mut EXEC_STACK: [u8; USER_STACK_BYTES] = [0; USER_STACK_BYTES];
static mut ELF_SCRATCH: [u8; ELF_SCRATCH_BYTES] = [0; ELF_SCRATCH_BYTES];

pub fn install(plan: UserFrameBackedImagePlan) {
    unsafe {
        core::ptr::write_volatile(core::ptr::addr_of_mut!(FRAME_PLAN), Some(plan));
        core::ptr::write_volatile(core::ptr::addr_of_mut!(STATE), READY);
    }
}

pub fn handle_user_syscall(frame: &mut TrapFrame) -> Option<TrapOutcome> {
    match frame.a7() {
        crate::syscall::SYS_CLONE => Some(handle_clone(frame)),
        crate::syscall::SYS_EXECVE => Some(handle_execve(frame)),
        crate::syscall::SYS_WAIT4 => Some(handle_wait4(frame)),
        crate::syscall::SYS_EXIT if state() == CHILD => Some(handle_child_exit(frame)),
        _ => None,
    }
}

fn state() -> u8 {
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(STATE)) }
}

fn plan() -> Option<UserFrameBackedImagePlan> {
    unsafe { core::ptr::read_volatile(core::ptr::addr_of!(FRAME_PLAN)) }
}

fn handle_clone(frame: &mut TrapFrame) -> TrapOutcome {
    if state() != READY || plan().is_none() {
        frame.set_a0(usize::MAX);
        frame.sepc = frame.sepc.wrapping_add(4);
        return TrapOutcome::Resume;
    }

    let mut parent = *frame;
    parent.set_a0(CHILD_PID);
    parent.sepc = parent.sepc.wrapping_add(4);
    unsafe {
        core::ptr::write_volatile(core::ptr::addr_of_mut!(PARENT_FRAME), parent);
        core::ptr::write_volatile(core::ptr::addr_of_mut!(STATE), CHILD);
    }
    frame.set_a0(0);
    frame.sepc = frame.sepc.wrapping_add(4);
    console::puts("[init] spawned shell pid=2\n");
    TrapOutcome::Resume
}

fn handle_execve(frame: &mut TrapFrame) -> TrapOutcome {
    if state() != CHILD || !user_path_is(frame.a0(), b"/bin/sh\0") {
        frame.set_a0(usize::MAX);
        frame.sepc = frame.sepc.wrapping_add(4);
        return TrapOutcome::Resume;
    }

    let Some(plan) = plan() else {
        return TrapOutcome::Fatal(crate::trap::SCAUSE_USER_ECALL);
    };
    unsafe {
        let source =
            core::slice::from_raw_parts(plan.stack_frame_start_pa as *const u8, USER_STACK_BYTES);
        let saved = &mut *core::ptr::addr_of_mut!(SAVED_PARENT_STACK);
        saved.copy_from_slice(source);
    }

    let Some(shell_frame) = load_image("/bin/sh", &["/bin/sh"], &["PATH=/bin"], plan) else {
        frame.set_a0(usize::MAX);
        frame.sepc = frame.sepc.wrapping_add(4);
        return TrapOutcome::Resume;
    };
    *frame = shell_frame;
    console::puts("[init-child] execve /bin/sh\n");
    TrapOutcome::Resume
}

fn handle_child_exit(frame: &mut TrapFrame) -> TrapOutcome {
    let code = frame.a0() as i32;
    let Some(plan) = plan() else {
        return TrapOutcome::Fatal(crate::trap::SCAUSE_USER_ECALL);
    };
    if !restore_init(plan) {
        return TrapOutcome::Fatal(crate::trap::SCAUSE_USER_ECALL);
    }
    *frame = unsafe { core::ptr::read_volatile(core::ptr::addr_of!(PARENT_FRAME)) };
    unsafe {
        core::ptr::write_volatile(core::ptr::addr_of_mut!(CHILD_EXIT_STATUS), code);
        core::ptr::write_volatile(core::ptr::addr_of_mut!(STATE), ZOMBIE);
    }
    console::puts("[init] child shell exited status=");
    put_status(code);
    console::puts("\n");
    TrapOutcome::Resume
}

fn handle_wait4(frame: &mut TrapFrame) -> TrapOutcome {
    if state() != ZOMBIE || frame.a0() as isize != CHILD_PID as isize {
        frame.set_a0(usize::MAX);
        frame.sepc = frame.sepc.wrapping_add(4);
        return TrapOutcome::Resume;
    }
    let status_ptr = frame.a1();
    let exit_status = unsafe {
        core::ptr::read_volatile(core::ptr::addr_of!(CHILD_EXIT_STATUS))
    };
    if status_ptr != 0 {
        user_access::with_user_memory_access(|| unsafe {
            (status_ptr as *mut i32).write(exit_status);
        });
    }
    frame.set_a0(CHILD_PID);
    frame.sepc = frame.sepc.wrapping_add(4);
    unsafe {
        core::ptr::write_volatile(core::ptr::addr_of_mut!(STATE), REAPED);
    }
    console::puts("[chapter] ch111 init child shell wait\n");
    TrapOutcome::Resume
}

fn load_image(
    path: &str,
    argv: &[&str],
    envp: &[&str],
    plan: UserFrameBackedImagePlan,
) -> Option<TrapFrame> {
    let dev = VirtioBlockDevice::probe().ok()?;
    let start_lba = block::mbr_first_partition_lba(&dev).ok()?;
    let root = PartitionBlockDevice::new("drive0p1", &dev, start_lba);
    let vfs = Vfs::mount_ext4(&root).ok()?;
    let scratch = unsafe { &mut *core::ptr::addr_of_mut!(ELF_SCRATCH) };
    let loaded = user_exec::load_init_exec_image(&vfs, path, scratch, plan.stack_top).ok()?;
    let exec_plan = rebind_plan_to_image(plan, &loaded)?;
    let stack = unsafe { &mut *core::ptr::addr_of_mut!(EXEC_STACK) };
    let launch =
        user_launch::prepare_user_init_launch::<4, 4>(&loaded, 0, stack, argv, envp).ok()?;
    unsafe {
        user_exec::materialize_frame_backed_user_image_to_frames(&loaded, &exec_plan, stack)
            .ok()?;
        core::arch::asm!("fence.i");
    }
    Some(launch.trapframe)
}

fn rebind_plan_to_image(
    mut plan: UserFrameBackedImagePlan,
    loaded: &crate::elf_loader::LoadedUserImage<'_>,
) -> Option<UserFrameBackedImagePlan> {
    if loaded.segment_count != plan.segment_count {
        return None;
    }
    plan.entry_vaddr = loaded.entry;
    let mut idx = 0usize;
    while idx < loaded.segment_count {
        let loaded_segment = loaded.segments[idx]?;
        let mut frame_segment = plan.segments[idx]?;
        if loaded_segment.vaddr & !(crate::sv39::PAGE_SIZE - 1)
            != frame_segment.vaddr & !(crate::sv39::PAGE_SIZE - 1)
        {
            return None;
        }
        frame_segment.vaddr = loaded_segment.vaddr;
        frame_segment.file_size = loaded_segment.file_size;
        frame_segment.mem_size = loaded_segment.mem_size;
        frame_segment.flags = loaded_segment.flags;
        plan.segments[idx] = Some(frame_segment);
        idx += 1;
    }
    Some(plan)
}

fn restore_init(plan: UserFrameBackedImagePlan) -> bool {
    let dev = match VirtioBlockDevice::probe() {
        Ok(dev) => dev,
        Err(_) => return false,
    };
    let start_lba = match block::mbr_first_partition_lba(&dev) {
        Ok(lba) => lba,
        Err(_) => return false,
    };
    let root = PartitionBlockDevice::new("drive0p1", &dev, start_lba);
    let vfs = match Vfs::mount_ext4(&root) {
        Ok(vfs) => vfs,
        Err(_) => return false,
    };
    let scratch = unsafe { &mut *core::ptr::addr_of_mut!(ELF_SCRATCH) };
    let loaded = match user_exec::load_init_exec_image(&vfs, "/sbin/init", scratch, plan.stack_top)
    {
        Ok(loaded) => loaded,
        Err(_) => return false,
    };
    let saved = unsafe { &*core::ptr::addr_of!(SAVED_PARENT_STACK) };
    let result =
        unsafe { user_exec::materialize_frame_backed_user_image_to_frames(&loaded, &plan, saved) };
    if result.is_ok() {
        unsafe { core::arch::asm!("fence.i") };
        true
    } else {
        false
    }
}

fn user_path_is(ptr: usize, expected: &[u8]) -> bool {
    if ptr == 0 {
        return false;
    }
    user_access::with_user_memory_access(|| unsafe {
        let actual = core::slice::from_raw_parts(ptr as *const u8, expected.len());
        actual == expected
    })
}

fn put_status(code: i32) {
    if code == 0 {
        console::puts("0");
    } else {
        console::puts("nonzero");
    }
}
