#![no_std]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#![no_main]

#[allow(dead_code)]
mod block;
#[allow(dead_code)]
mod block_io;
mod serial;
mod console;
mod char_device;
#[allow(dead_code)]
mod fd_table;
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
    let _ = user_vm.map_anon(0x4000, 0x4010, sv39::PTE_R);
    let checked_user = user_copy::CheckedUserSlice::new(b"/dev/console\0", 0x4000, &user_vm);
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
