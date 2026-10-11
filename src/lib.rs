#![cfg_attr(not(test), no_std)]
// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0


pub mod block;
pub mod ramdisk;
pub mod block_io;
pub mod char_device;
pub mod serial;
pub mod serial_rx;
pub mod console;
pub mod context_switch;
pub mod demo_user_init;
pub mod device;
pub mod devfs;
pub mod elf_loader;
pub mod ext4;
pub mod exec_args;
pub mod exec_path;
pub mod exec_flow;
pub mod fd_table;
pub mod irq_guard;
pub mod idle;
pub mod kheap;
pub mod page_fault;
pub mod page_frame_allocator;
pub mod pmm;
pub mod process_model;
pub mod process_io;
pub mod process_runtime;
pub mod sbi;
pub mod scheduler;
pub mod sv39;
pub mod syscall;
pub mod task_vm;
pub mod trap;
pub mod trap_runtime;
pub mod user_copy;
pub mod uart;
pub mod user_access;
pub mod user_exec;
pub mod user_init;
pub mod user_launch;
pub mod user_process;
pub mod user_stack;
pub mod user_vm;
pub mod virtio_blk;
pub mod vfs;
pub mod vm;
pub mod wait_queue;

pub fn align_up(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (value + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::align_up;
    use super::block::{BlockDevice, BlockError};
    use super::ramdisk::RamDisk;
    use super::block_io::read_bytes;
    use super::char_device::{CharError, CharacterDevice};
    use super::console;
    use super::context_switch::switch_context;
    use super::device::{Device, DeviceKind};
    use super::devfs::{DeviceNamespace, VfsLookupError, VfsNodeKind, VfsNodeTarget};
    use super::elf_loader::{
        load_init_from_vfs, load_user_elf, parse_elf_header, pflags_to_pte, ElfError,
    };
    use super::ext4::{Ext4Fs, FsError};
    use super::exec_args::{
        copyin_execv_args, materialize_execv_stack, ExecvArgBlock, ExecvArgError,
    };
    use super::exec_path::load_exec_image_from_args;
    use super::exec_flow::apply_execv_from_user_args;
    use super::fd_table::{stdio_table, stdio_table_with_capacity, FileDescriptorTable, FileError, OpenFile, OpenMode};
    use super::idle::{idle_action_for_schedule_decision, IdleAction};
    use super::kheap::init_kheap;
    use super::page_fault::{
        handle_user_page_fault, PageFaultResult, SCAUSE_LOAD_PAGE_FAULT, SCAUSE_STORE_PAGE_FAULT,
    };
    use super::page_frame_allocator::PageFrameAllocator;
    use super::pmm::{PageFrameManager, PmmError};
    use super::process_model::{
        exit_process_from_task, exit_process_from_task_with_scheduler, ProcError, ProcState,
        reap_exited_process, wait_reap_zombie_child, ProcessTable, ProcessWaitReapRuntime,
        TaskControlBlock, TaskTable,
    };
    use super::process_io::{
        CurrentProcessIoContext, ProcessIoContext, ProcessIoRuntime, ProcessIoTable,
    };
    use super::process_runtime::run_fork_exec_wait_demo;
    use super::scheduler::{ScheduleDecision, Scheduler, TaskState};
    use super::serial::{SerialDevice, UART0_BASE};
    use super::serial_rx::{wake_reader_on_rx_event, SerialRxEvent};
    use super::sv39::{
        make_satp, PageTableEntry, RootPageTable, PTE_A, PTE_D, PTE_G, PTE_R, PTE_U, PTE_V, PTE_W, PTE_X,
    };
    use super::task_vm::{
        address_space_switch_for_decision, address_space_switch_for_decision_with_spaces,
        apply_address_space_switch, commit_satp_hardware,
        prepare_rooted_satp_commit_sequence, prepare_rooted_user_launch_csr_sequence,
        prepare_satp_commit, prepare_satp_hardware_commit,
        prepare_user_launch_csr_sequence, probe_satp_hardware_commit_roundtrip,
        SatpHardwareCommitError,
    };
    use super::syscall::{
        dispatch_current_execv_frame_from_runtime, dispatch_current_execv_syscall, dispatch_current_execv_syscall_from_runtime,
        decode_syscall, handle_current_process_blocking_read_checked_user_syscall,
        handle_current_process_close, handle_current_process_read_checked_user_syscall,
        handle_current_process_write_checked_user_syscall,
        handle_current_process_clone_syscall, handle_current_process_execv_checked_user_syscall, handle_process_execv_checked_user_syscall, BlockingReadResult, CurrentExecvDispatchContext, CurrentExecvRuntimeContext,
        handle_process_execv_syscall, handle_process_fork_syscall,
        handle_process_wait4_checked_user_syscall,
        handle_open_path, handle_process_open_checked_user_path,
        handle_process_open_path, handle_process_open_user_path, handle_process_write,
        handle_process_write_checked_user_buf, handle_process_write_checked_user_syscall,
        handle_process_write_user_buf, handle_syscall, open_mode_from_flags, process_wait_errno,
        Syscall, SyscallResult, EACCES, EAGAIN, EBADF, ECHILD, EFAULT, SYS_CLOSE, SYS_EXIT, SYS_CLONE, SYS_EXECVE,
        SYS_OPENAT, SYS_READ, SYS_WAIT4, SYS_WRITE,
    };
    use super::trap::{
        apply_schedule_decision, dispatch_trap_outcome,
        dispatch_exit_outcome_with_process_runtime, dispatch_trap_outcome_with_global_runtime,
        handle_blocking_syscall_result, handle_current_execv_frame_from_runtime,
        handle_syscall_result, handle_timer_tick_with_scheduler, handle_trap,
        handle_trap_with_current_execv_runtime, handle_trap_with_execv_resources,
        handle_user_page_fault_outcome, prepare_enter_user_scratch,
        prepare_kernel_trap_stack_switch,
        prepare_trap_entry_scratch, simulate_trap_entry_return_scratch,
        simulate_trap_entry_stack_swap,
        schedule_after_blocked_syscall, KernelTrapStackError, KernelTrapStackSwitch,
        ExecvTrapResourceStore, TrapDispatchResult, TrapFrame, TrapOutcome, SCAUSE_USER_ECALL,
        TRAP_FRAME_SIZE,
    };
    use super::trap_runtime::{
        install_initial_user_task, reset_global_trap_runtime, switched_tids,
        with_global_trap_runtime, ExecvTrapDispatchState,
        ProcessTrapRuntime, ProcessWaitSyscallRuntime, TrapRuntime,
    };
    use super::user_copy::{CheckedUserSlice, CheckedUserSliceMut, UserCopyError, UserSlice, UserSliceMut};
    use super::user_exec::{
        install_exec_context, load_init_exec_image, materialize_frame_backed_user_image,
        materialize_segments, ExecError, FrameBackedMaterializeError, FrameRegionMut,
        SegmentPlacement, UserExecImage, UserRegionMut,
    };
    use super::user_access::{
        enable_user_memory_access_status, restore_user_memory_access_status,
        user_access_window_from_status, user_memory_access_enabled, SSTATUS_SUM,
    };
    use super::user_init::{build_user_init_context, run_user_init_demo, SSTATUS_SPP};
    use super::user_launch::prepare_user_init_launch;
    use super::user_process::create_user_process_from_launch;
    use super::user_stack::{build_initial_user_stack, UserStackError};
    use super::user_vm::{
        build_user_vm_plan, guard_kernel_runtime_after_satp, install_kernel_identity_mapping,
        install_frame_backed_user_mapping_in_tables, install_user_vm_plan_in_root,
        is_kernel_mapping, is_leaf_user_mapping, is_table_mapping,
        plan_frame_backed_user_image, preflight_satp_commit, vpn0, vpn1, vpn2,
        KernelRuntimeSatpGuard,
        KernelRuntimeSatpGuardError, ProcessAddressSpace, ProcessAddressSpaceTable,
        SatpCommitPreflight, SatpCommitPreflightError, UserFrameBackedImageError,
    };
    use super::vfs::Vfs;
    use super::vm::{FaultAccess, VmError, VmSpace};
    use super::wait_queue::WaitQueue;


    struct MockCharDevice {
        input: std::cell::RefCell<std::vec::Vec<u8>>,
        written: std::cell::RefCell<std::vec::Vec<u8>>,
    }

    impl MockCharDevice {
        fn new(input: &[u8]) -> Self {
            Self {
                input: std::cell::RefCell::new(input.to_vec()),
                written: std::cell::RefCell::new(std::vec::Vec::new()),
            }
        }

        fn written(&self) -> std::vec::Vec<u8> {
            self.written.borrow().clone()
        }
    }

    impl Device for MockCharDevice {
        fn name(&self) -> &str {
            "mocktty0"
        }

        fn kind(&self) -> DeviceKind {
            DeviceKind::Char
        }
    }

    impl CharacterDevice for MockCharDevice {
        fn read(&self, out: &mut [u8]) -> Result<usize, CharError> {
            if out.is_empty() {
                return Err(CharError::BadBuffer);
            }
            let mut input = self.input.borrow_mut();
            if input.is_empty() {
                return Err(CharError::WouldBlock);
            }
            let n = core::cmp::min(out.len(), input.len());
            for (dst, src) in out.iter_mut().zip(input.drain(..n)) {
                *dst = src;
            }
            Ok(n)
        }

        fn write(&self, bytes: &[u8]) -> Result<usize, CharError> {
            if bytes.is_empty() {
                return Err(CharError::BadBuffer);
            }
            self.written.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }
    }

    #[test]
    fn fd_table_preopens_stdio_to_console_slots() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"in")));
        let table = stdio_table(dev);

        assert!(table.get(0).is_ok());
        assert!(table.get(1).is_ok());
        assert!(table.get(2).is_ok());
        assert_eq!(table.write(1, b"stdout"), Ok(6));
        assert_eq!(table.write(2, b"stderr"), Ok(6));
        assert_eq!(dev.written(), b"stdoutstderr".to_vec());
        assert_eq!(table.write(0, b"nope"), Err(FileError::NotWritable));
        assert_eq!(table.write(3, b"nope"), Err(FileError::BadFd));
    }

    #[test]
    fn fd_table_allocates_lowest_free_descriptor() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let mut table = FileDescriptorTable::<4>::new();

        assert_eq!(table.allocate(OpenFile::write_only_character(dev)), Ok(0));
        assert_eq!(table.allocate(OpenFile::write_only_character(dev)), Ok(1));
        assert_eq!(table.write(1, b"fd1"), Ok(3));
        assert_eq!(dev.written(), b"fd1".to_vec());
    }

    #[test]
    fn dev_namespace_resolves_root_dev_and_console() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);

        assert_eq!(ns.lookup_path("/").unwrap().kind(), VfsNodeKind::Directory);
        assert_eq!(ns.lookup_path("/dev").unwrap().kind(), VfsNodeKind::Directory);
        let console = ns.lookup_path("/dev/console").unwrap();
        assert_eq!(console.path(), "/dev/console");
        assert_eq!(console.name(), "console");
        assert_eq!(console.kind(), VfsNodeKind::CharacterDevice);
        match console.target() {
            VfsNodeTarget::Character(ch) => assert_eq!(ch.name(), "mocktty0"),
            _ => panic!("console target should be a character device"),
        }
    }

    #[test]
    fn dev_namespace_rejects_bad_or_nested_device_paths() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);

        assert_eq!(ns.lookup_path("dev/console").err(), Some(VfsLookupError::BadPath));
        assert_eq!(ns.lookup_path("/missing").err(), Some(VfsLookupError::NotFound));
        assert_eq!(ns.lookup_path("/dev/console/extra").err(), Some(VfsLookupError::NotDirectory));
        assert_eq!(ns.lookup_path("/dev/vda").err(), Some(VfsLookupError::NotFound));
    }

    #[test]
    fn dev_namespace_can_expose_raw_block_node() {
        let ch: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let image: &'static [u8; 1024] = std::boxed::Box::leak(std::boxed::Box::new([0u8; 1024]));
        let blk: &'static RamDisk = std::boxed::Box::leak(std::boxed::Box::new(RamDisk::new("ramblk0", image, 512)));
        let ns = DeviceNamespace::with_block0(ch, blk);

        let node = ns.lookup_path("/dev/vda").unwrap();
        assert_eq!(node.path(), "/dev/vda");
        assert_eq!(node.kind(), VfsNodeKind::BlockDevice);
        match node.target() {
            VfsNodeTarget::Block(block) => assert_eq!(block.name(), "ramblk0"),
            _ => panic!("vda target should be a block device"),
        }
    }

    #[test]
    fn fd_table_can_open_console_device_node() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let node = ns.lookup_path("/dev/console").unwrap();
        let file = OpenFile::from_vfs_node(node, OpenMode::WriteOnly).unwrap();

        assert_eq!(file.write(b"via-dev-node"), Ok(12));
        assert_eq!(dev.written(), b"via-dev-node".to_vec());
    }

    #[test]
    fn fd_table_open_path_allocates_console_fd() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut table = stdio_table_with_capacity::<8>(dev);

        let fd = table.open_path(&ns, "/dev/console", OpenMode::WriteOnly).unwrap();
        assert_eq!(fd, 3);
        assert_eq!(table.write(fd, b"opened"), Ok(6));
        assert_eq!(dev.written(), b"opened".to_vec());
    }

    #[test]
    fn syscall_open_path_returns_allocated_fd() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut table = stdio_table_with_capacity::<8>(dev);

        assert_eq!(
            handle_open_path(&mut table, &ns, "/dev/console", 1),
            SyscallResult::Continue(3)
        );
        assert_eq!(table.write(3, b"sys-open"), Ok(8));
        assert_eq!(dev.written(), b"sys-open".to_vec());
    }

    #[test]
    fn syscall_open_path_rejects_missing_or_unsupported_nodes() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut table = stdio_table(dev);

        assert_eq!(
            handle_open_path(&mut table, &ns, "/dev/missing", 1),
            SyscallResult::Continue(-1)
        );
        assert_eq!(open_mode_from_flags(3), None);
    }

    #[test]
    fn process_io_context_keeps_open_fd_for_later_write() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut io = ProcessIoContext::<8>::with_stdio(dev);

        let fd = io.open_path(&ns, "/dev/console", OpenMode::WriteOnly).unwrap();
        assert_eq!(fd, 3);
        assert_eq!(io.write_fd(fd, b"persistent-fd"), Ok(13));
        assert_eq!(dev.written(), b"persistent-fd".to_vec());
    }

    #[test]
    fn syscall_helpers_use_process_io_context() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut io = ProcessIoContext::<8>::with_stdio(dev);

        let fd = match handle_process_open_path(&mut io, &ns, "/dev/console", 1) {
            SyscallResult::Continue(fd) => fd as usize,
            other => panic!("unexpected open result: {:?}", other),
        };
        assert_eq!(fd, 3);
        assert_eq!(
            handle_process_write(&io, fd, b"through-process"),
            SyscallResult::Continue(15)
        );
        assert_eq!(dev.written(), b"through-process".to_vec());
    }

    #[test]
    fn user_copy_reads_bytes_and_c_strings() {
        let user = UserSlice::new(b"abc/dev/console\0tail", 0x4000);
        let mut buf = [0u8; 3];
        assert_eq!(user.copyin(0x4000, &mut buf), Ok(3));
        assert_eq!(&buf, b"abc");

        let mut path = [0u8; 32];
        assert_eq!(user.copyin_cstr(0x4003, &mut path), Ok("/dev/console"));
    }

    #[test]
    fn user_copy_rejects_bad_ranges_and_missing_nul() {
        let user = UserSlice::new(b"abc", 0x8000);
        let mut buf = [0u8; 4];
        assert_eq!(user.copyin_bytes(0x8000, 4, &mut buf), Err(UserCopyError::BadAddress));
        assert_eq!(user.copyin_cstr(0x8000, &mut buf), Err(UserCopyError::BadAddress));
        assert_eq!(user.copyin_cstr(0x8000, &mut [0u8; 3]), Err(UserCopyError::MissingNul));
    }

    #[test]
    fn syscall_open_uses_copyin_path_before_fd_allocation() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let user = UserSlice::new(b"/dev/console\0", 0x5000);
        let mut io = ProcessIoContext::<8>::with_stdio(dev);
        let mut scratch = [0u8; 32];

        assert_eq!(
            handle_process_open_user_path(&mut io, &ns, &user, 0x5000, 1, &mut scratch),
            SyscallResult::Continue(3)
        );
        assert_eq!(io.write_fd(3, b"copied-path"), Ok(11));
        assert_eq!(dev.written(), b"copied-path".to_vec());
    }

    #[test]
    fn user_memory_access_status_window_enables_sum_then_restores() {
        let previous = 0x20;
        let enabled = enable_user_memory_access_status(previous);
        let restored = restore_user_memory_access_status(previous);
        let window = user_access_window_from_status(previous);

        assert_eq!(enabled, previous | SSTATUS_SUM);
        assert!(user_memory_access_enabled(enabled));
        assert_eq!(restored, previous);
        assert_eq!(window.enabled_sstatus, enabled);
        assert_eq!(window.restored_sstatus, restored);
    }

    #[test]
    fn syscall_write_uses_copyin_buffer_before_device_write() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let mut io = ProcessIoContext::<8>::with_stdio(dev);
        let ns = DeviceNamespace::new(dev);
        let fd = io.open_path(&ns, "/dev/console", OpenMode::WriteOnly).unwrap();
        let user = UserSlice::new(b"hello from user", 0x6000);
        let mut scratch = [0u8; 32];

        assert_eq!(
            handle_process_write_user_buf(&io, &user, fd, 0x6000, 15, &mut scratch),
            SyscallResult::Continue(15)
        );
        assert_eq!(dev.written(), b"hello from user".to_vec());
    }

    #[test]
    fn vm_space_allows_read_ranges_by_area_permissions() {
        let mut vm = VmSpace::<4>::new();
        vm.map_anon(0x4000, 0x4010, PTE_R).unwrap();
        vm.map_anon(0x5000, 0x5010, PTE_W).unwrap();

        assert!(vm.allows_range(0x4000, 16, FaultAccess::Read));
        assert!(!vm.allows_range(0x4008, 16, FaultAccess::Read));
        assert!(!vm.allows_range(0x5000, 1, FaultAccess::Read));
        assert!(vm.allows_range(0x5000, 1, FaultAccess::Write));
    }

    #[test]
    fn checked_user_copy_requires_vm_read_permission() {
        let mut vm = VmSpace::<4>::new();
        vm.map_anon(0x7000, 0x7010, PTE_R).unwrap();
        let user = CheckedUserSlice::new(b"/dev/console\0", 0x7000, &vm);
        let mut path = [0u8; 32];

        assert_eq!(user.copyin_cstr(0x7000, &mut path), Ok("/dev/console"));
        assert_eq!(user.copyin_bytes(0x7000, 20, &mut [0u8; 32]), Err(UserCopyError::Permission));
    }

    #[test]
    fn checked_user_copy_rejects_write_only_vm_area() {
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0x8000, 0x8010, PTE_W).unwrap();
        let user = CheckedUserSlice::new(b"abc\0", 0x8000, &vm);
        assert_eq!(user.copyin_cstr(0x8000, &mut [0u8; 8]), Err(UserCopyError::Permission));
    }

    #[test]
    fn syscall_checked_copyin_opens_and_writes_user_buffers() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut vm = VmSpace::<4>::new();
        vm.map_anon(0x9000, 0x9020, PTE_R).unwrap();
        let user = CheckedUserSlice::new(b"/dev/console\0hello", 0x9000, &vm);
        let mut io = ProcessIoContext::<8>::with_stdio(dev);
        let mut path_scratch = [0u8; 32];
        let mut write_scratch = [0u8; 16];

        let fd = match handle_process_open_checked_user_path(
            &mut io,
            &ns,
            &user,
            0x9000,
            1,
            &mut path_scratch,
        ) {
            SyscallResult::Continue(fd) => fd as usize,
            other => panic!("unexpected open result: {:?}", other),
        };
        assert_eq!(
            handle_process_write_checked_user_buf(&io, &user, fd, 0x900d, 5, &mut write_scratch),
            SyscallResult::Continue(5)
        );
        assert_eq!(dev.written(), b"hello".to_vec());
    }


    #[test]
    fn fd_write_syscall_uses_checked_copyin_and_process_fd_table() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut io = ProcessIoContext::<8>::with_stdio(dev);
        let fd = io.open_path(&ns, "/dev/console", OpenMode::WriteOnly).unwrap();
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0xa000, 0xa020, PTE_R).unwrap();
        let user = CheckedUserSlice::new(b"fd-syscall", 0xa000, &vm);
        let mut scratch = [0u8; 16];

        assert_eq!(
            handle_process_write_checked_user_syscall(&io, &user, fd, 0xa000, 10, &mut scratch),
            SyscallResult::Continue(10)
        );
        assert_eq!(dev.written(), b"fd-syscall".to_vec());
    }

    #[test]
    fn fd_write_syscall_maps_fd_and_user_errors_to_errno() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut io = ProcessIoContext::<8>::with_stdio(dev);
        let fd = io.open_path(&ns, "/dev/console", OpenMode::WriteOnly).unwrap();
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0xb000, 0xb010, PTE_R).unwrap();
        let user = CheckedUserSlice::new(b"errno", 0xb000, &vm);
        let mut scratch = [0u8; 8];

        assert_eq!(
            handle_process_write_checked_user_syscall(&io, &user, 7, 0xb000, 5, &mut scratch),
            SyscallResult::Continue(-EBADF)
        );
        assert_eq!(
            handle_process_write_checked_user_syscall(&io, &user, 0, 0xb000, 5, &mut scratch),
            SyscallResult::Continue(-EACCES)
        );
        assert_eq!(
            handle_process_write_checked_user_syscall(&io, &user, fd, 0xb008, 12, &mut scratch),
            SyscallResult::Continue(-EFAULT)
        );
        assert_eq!(dev.written(), std::vec::Vec::<u8>::new());
    }


    #[test]
    fn current_process_io_context_keeps_pid_and_fd_state() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut current = CurrentProcessIoContext::<8>::with_stdio(42, dev);

        assert_eq!(current.pid(), 42);
        let fd = current.open_path(&ns, "/dev/console", OpenMode::WriteOnly).unwrap();
        assert_eq!(fd, 3);
        assert_eq!(current.write_fd(fd, b"current-io"), Ok(10));
        assert_eq!(dev.written(), b"current-io".to_vec());
    }

    #[test]
    fn current_process_write_syscall_uses_current_io_context() {
        let dev: &'static MockCharDevice = std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut current = CurrentProcessIoContext::<8>::with_stdio(1, dev);
        let fd = current.open_path(&ns, "/dev/console", OpenMode::WriteOnly).unwrap();
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0xc000, 0xc020, PTE_R).unwrap();
        let user = CheckedUserSlice::new(b"from-current", 0xc000, &vm);
        let mut scratch = [0u8; 16];

        assert_eq!(
            handle_current_process_write_checked_user_syscall(
                &current,
                &user,
                fd,
                0xc000,
                12,
                &mut scratch,
            ),
            SyscallResult::Continue(12)
        );
        assert_eq!(dev.written(), b"from-current".to_vec());
    }


    #[test]
    fn process_io_table_resolves_current_io_from_task_owner() {
        let dev: &'static MockCharDevice =
            std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let mut table = ProcessIoTable::<2, 8>::new();
        table.insert_stdio(42, dev).unwrap();
        let task = TaskControlBlock::new(7, 42, TrapFrame::default());

        let current = table.current_for_task(&task).unwrap();
        assert_eq!(current.pid(), 42);
    }

    #[test]
    fn task_owned_current_io_write_syscall_uses_owner_pid_lookup() {
        let dev: &'static MockCharDevice =
            std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut table = ProcessIoTable::<2, 8>::new();
        table.insert_stdio(1, dev).unwrap();
        let task = TaskControlBlock::new(1, 1, TrapFrame::default());
        let fd = table
            .current_for_task_mut(&task)
            .unwrap()
            .open_path(&ns, "/dev/console", OpenMode::WriteOnly)
            .unwrap();
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0xd000, 0xd020, PTE_R).unwrap();
        let user = CheckedUserSlice::new(b"task-owned", 0xd000, &vm);
        let mut scratch = [0u8; 16];

        let current = table.current_for_task(&task).unwrap();
        assert_eq!(
            handle_current_process_write_checked_user_syscall(
                current,
                &user,
                fd,
                0xd000,
                10,
                &mut scratch,
            ),
            SyscallResult::Continue(10)
        );
        assert_eq!(dev.written(), b"task-owned".to_vec());
    }


    #[test]
    fn process_io_runtime_creates_process_with_stdio() {
        let dev: &'static MockCharDevice =
            std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let mut runtime = ProcessIoRuntime::<2, 2, 2, 8>::new();
        let pid = runtime
            .create_process_with_stdio(None, VmSpace::<2>::new(), dev)
            .unwrap();
        let task = TaskControlBlock::new(1, pid, TrapFrame::default());

        assert_eq!(pid, 1);
        assert_eq!(runtime.current_for_task(&task).unwrap().pid(), pid);
        assert!(runtime.processes_mut().get_mut(pid).is_ok());
    }

    #[test]
    fn process_owned_io_runtime_write_uses_created_process_context() {
        let dev: &'static MockCharDevice =
            std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut runtime = ProcessIoRuntime::<2, 2, 2, 8>::new();
        let pid = runtime
            .create_process_with_stdio(None, VmSpace::<2>::new(), dev)
            .unwrap();
        let task = TaskControlBlock::new(1, pid, TrapFrame::default());
        let fd = runtime
            .current_for_task_mut(&task)
            .unwrap()
            .open_path(&ns, "/dev/console", OpenMode::WriteOnly)
            .unwrap();
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0xe000, 0xe020, PTE_R).unwrap();
        let user = CheckedUserSlice::new(b"owned-io", 0xe000, &vm);
        let mut scratch = [0u8; 16];

        let current = runtime.current_for_task(&task).unwrap();
        assert_eq!(
            handle_current_process_write_checked_user_syscall(
                current,
                &user,
                fd,
                0xe000,
                8,
                &mut scratch,
            ),
            SyscallResult::Continue(8)
        );
        assert_eq!(dev.written(), b"owned-io".to_vec());
    }


    #[test]
    fn fd_table_close_releases_slot_and_reuses_lowest_fd() {
        let dev: &'static MockCharDevice =
            std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut table = stdio_table_with_capacity::<8>(dev);
        let fd = table.open_path(&ns, "/dev/console", OpenMode::WriteOnly).unwrap();

        assert_eq!(fd, 3);
        assert_eq!(table.close(fd), Ok(()));
        assert_eq!(table.write(fd, b"closed"), Err(FileError::BadFd));
        assert_eq!(table.open_path(&ns, "/dev/console", OpenMode::WriteOnly), Ok(fd));
    }

    #[test]
    fn close_syscall_closes_current_process_fd() {
        let dev: &'static MockCharDevice =
            std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"")));
        let ns = DeviceNamespace::new(dev);
        let mut runtime = ProcessIoRuntime::<2, 2, 2, 8>::new();
        let pid = runtime
            .create_process_with_stdio(None, VmSpace::<2>::new(), dev)
            .unwrap();
        let task = TaskControlBlock::new(1, pid, TrapFrame::default());
        let fd = runtime
            .current_for_task_mut(&task)
            .unwrap()
            .open_path(&ns, "/dev/console", OpenMode::WriteOnly)
            .unwrap();
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0xf000, 0xf020, PTE_R).unwrap();
        let user = CheckedUserSlice::new(b"after-close", 0xf000, &vm);
        let mut scratch = [0u8; 16];
        let current = runtime.current_for_task_mut(&task).unwrap();

        assert_eq!(handle_current_process_close(current, fd), SyscallResult::Continue(0));
        assert_eq!(handle_current_process_close(current, fd), SyscallResult::Continue(-EBADF));
        assert_eq!(
            handle_current_process_write_checked_user_syscall(
                current,
                &user,
                fd,
                0xf000,
                11,
                &mut scratch,
            ),
            SyscallResult::Continue(-EBADF)
        );
        assert_eq!(dev.written(), std::vec::Vec::<u8>::new());
    }

    #[test]
    fn syscall_decodes_close_number() {
        assert_eq!(decode_syscall(SYS_CLOSE, 3, 0, 0), Some(Syscall::Close { fd: 3 }));
    }


    #[test]
    fn user_copyout_writes_bytes_to_user_buffer() {
        let mut bytes = [0u8; 8];
        {
            let mut user = UserSliceMut::new(&mut bytes, 0x1000);
            assert_eq!(user.copyout_bytes(0x1002, b"abc"), Ok(3));
        }
        assert_eq!(bytes, [0, 0, b'a', b'b', b'c', 0, 0, 0]);

        let mut user = UserSliceMut::new(&mut bytes, 0x1000);
        assert_eq!(user.copyout_bytes(0x1006, b"abc"), Err(UserCopyError::BadAddress));
    }

    #[test]
    fn checked_user_copyout_requires_vm_write_permission() {
        let mut bytes = [0u8; 8];
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0x2000, 0x2008, PTE_W).unwrap();
        let mut user = CheckedUserSliceMut::new(&mut bytes, 0x2000, &vm);

        assert_eq!(user.copyout_bytes(0x2000, b"rw"), Ok(2));
        assert_eq!(&bytes[..2], b"rw");

        let mut ro_vm = VmSpace::<2>::new();
        ro_vm.map_anon(0x3000, 0x3008, PTE_R).unwrap();
        let mut ro_user = CheckedUserSliceMut::new(&mut bytes, 0x3000, &ro_vm);
        assert_eq!(ro_user.copyout_bytes(0x3000, b"x"), Err(UserCopyError::Permission));
    }


    #[test]
    fn execv_args_copyin_reads_path_argv_and_materializes_stack() {
        const BASE: usize = 0x5000_0000;
        const PATH: usize = BASE;
        const ARG0: usize = BASE + 0x40;
        const ARG1: usize = BASE + 0x60;
        const ARGV: usize = BASE + 0x100;
        let mut user_mem = [0u8; 512];
        write_cstr_at(&mut user_mem, PATH - BASE, b"/sbin/init");
        write_cstr_at(&mut user_mem, ARG0 - BASE, b"/sbin/init");
        write_cstr_at(&mut user_mem, ARG1 - BASE, b"--verbose");
        write_usize_at(&mut user_mem, ARGV - BASE, ARG0);
        write_usize_at(&mut user_mem, ARGV - BASE + core::mem::size_of::<usize>(), ARG1);
        write_usize_at(
            &mut user_mem,
            ARGV - BASE + 2 * core::mem::size_of::<usize>(),
            0,
        );

        let mut vm = VmSpace::<1>::new();
        vm.map_anon(BASE, BASE + user_mem.len(), PTE_R).unwrap();
        let user = CheckedUserSlice::new(&user_mem, BASE, &vm);
        let mut block = ExecvArgBlock::<4, 64>::new();
        let report = copyin_execv_args(&user, PATH, ARGV, &mut block).unwrap();

        assert_eq!(report.path_len, 10);
        assert_eq!(report.argc, 2);
        assert_eq!(report.first_arg_len, 10);
        assert_eq!(block.path_str().unwrap(), "/sbin/init");
        assert_eq!(block.arg_str(0).unwrap(), "/sbin/init");
        assert_eq!(block.arg_str(1).unwrap(), "--verbose");

        let stack_top = 0x4010_0000usize;
        let mut stack = [0u8; 512];
        let layout = materialize_execv_stack(&block, &mut stack, stack_top).unwrap();
        let base = stack_top - stack.len();
        let sp_off = layout.sp - base;
        assert_eq!(read_usize_at(&stack, sp_off), 2);
        let argv0 = read_usize_at(&stack, sp_off + core::mem::size_of::<usize>());
        let argv1 = read_usize_at(&stack, sp_off + 2 * core::mem::size_of::<usize>());
        let argv_null = read_usize_at(&stack, sp_off + 3 * core::mem::size_of::<usize>());
        let envp_null = read_usize_at(&stack, sp_off + 4 * core::mem::size_of::<usize>());
        assert_eq!(argv_null, 0);
        assert_eq!(envp_null, 0);
        assert_eq!(read_cstr_at(&stack, base, argv0), b"/sbin/init");
        assert_eq!(read_cstr_at(&stack, base, argv1), b"--verbose");
    }

    #[test]
    fn execv_args_copyin_rejects_too_many_args() {
        const BASE: usize = 0x6000_0000;
        let mut user_mem = [0u8; 280];
        write_cstr_at(&mut user_mem, 0, b"/sbin/init");
        write_cstr_at(&mut user_mem, 0x40, b"/sbin/init");
        write_cstr_at(&mut user_mem, 0x60, b"--verbose");
        write_usize_at(&mut user_mem, 0x100, BASE + 0x40);
        write_usize_at(&mut user_mem, 0x108, BASE + 0x60);
        write_usize_at(&mut user_mem, 0x110, BASE + 0x80);
        let mut vm = VmSpace::<1>::new();
        vm.map_anon(BASE, BASE + user_mem.len(), PTE_R).unwrap();
        let user = CheckedUserSlice::new(&user_mem, BASE, &vm);
        let mut block = ExecvArgBlock::<2, 64>::new();
        assert_eq!(
            copyin_execv_args(&user, BASE, BASE + 0x100, &mut block),
            Err(ExecvArgError::TooManyArgs)
        );
    }

    #[test]
    fn execv_args_path_loads_vfs_elf_exec_image() {
        const BASE: usize = 0x7000_0000;
        const PATH: usize = BASE;
        const ARG0: usize = BASE + 0x40;
        const ARGV: usize = BASE + 0x100;
        let mut user_mem = [0u8; 512];
        write_cstr_at(&mut user_mem, PATH - BASE, b"/sbin/init");
        write_cstr_at(&mut user_mem, ARG0 - BASE, b"/sbin/init");
        write_usize_at(&mut user_mem, ARGV - BASE, ARG0);
        write_usize_at(&mut user_mem, ARGV - BASE + core::mem::size_of::<usize>(), 0);

        let mut vm = VmSpace::<1>::new();
        vm.map_anon(BASE, BASE + user_mem.len(), PTE_R).unwrap();
        let user = CheckedUserSlice::new(&user_mem, BASE, &vm);
        let mut block = ExecvArgBlock::<4, 64>::new();
        copyin_execv_args(&user, PATH, ARGV, &mut block).unwrap();

        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut scratch = [0u8; 512];
        let (exec, report) =
            load_exec_image_from_args(&vfs, &block, &mut scratch, 0x4010_0000).unwrap();

        assert_eq!(report.path_len, 10);
        assert_eq!(report.argc, 1);
        assert_eq!(report.entry, 0x4000_0000);
        assert_eq!(report.stack_top, 0x4010_0000);
        assert_eq!(report.segment_count, 1);
        assert_eq!(exec.entry, report.entry);
        assert_eq!(exec.segment_count, 1);
        assert_eq!(exec.segments[0].unwrap().vaddr, 0x4000_0000);
    }

    #[test]
    fn execv_apply_pipeline_replaces_child_with_loaded_image_and_stack() {
        const BASE: usize = 0x7000_0000;
        const PATH: usize = BASE;
        const ARG0: usize = BASE + 0x40;
        const ARGV: usize = BASE + 0x100;
        let mut user_mem = [0u8; 512];
        write_cstr_at(&mut user_mem, PATH - BASE, b"/sbin/init");
        write_cstr_at(&mut user_mem, ARG0 - BASE, b"/sbin/init");
        write_usize_at(&mut user_mem, ARGV - BASE, ARG0);
        write_usize_at(&mut user_mem, ARGV - BASE + core::mem::size_of::<usize>(), 0);

        let mut user_vm = VmSpace::<1>::new();
        user_vm.map_anon(BASE, BASE + user_mem.len(), PTE_R).unwrap();
        let user = CheckedUserSlice::new(&user_mem, BASE, &user_vm);

        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut scratch = [0u8; 512];
        let mut stack = [0u8; 256];

        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let parent_pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let mut parent_frame = TrapFrame::default();
        parent_frame.sepc = 0x4000_0000;
        parent_frame.set_sp(0x4010_0000);
        runtime.add_runnable_task(1, parent_pid, parent_frame).unwrap();
        runtime.start_first_task();

        let mut child_frame = TrapFrame::default();
        child_frame.sepc = 0x4000_1000;
        child_frame.set_sp(0x4010_1000);
        let fork = runtime
            .fork_current_process(2, child_frame, VmSpace::<2>::new())
            .unwrap();

        let (exec, report) = apply_execv_from_user_args::<1, 4, 64, 4, 2, 2, 5>(
            &mut runtime,
            fork.child_tid,
            &user,
            PATH,
            ARGV,
            &vfs,
            &mut scratch,
            &mut stack,
            0x4010_0000,
            VmSpace::<2>::new(),
        )
        .unwrap();

        assert_eq!(report.path_len, 10);
        assert_eq!(report.argc, 1);
        assert_eq!(report.entry, 0x4000_0000);
        assert_eq!(report.image_stack_top, 0x4010_0000);
        assert_eq!(report.initial_sp, exec.stack_top);
        assert_eq!(report.segment_count, 1);
        assert_eq!(report.pid, fork.child_pid);
        assert_eq!(report.tid, fork.child_tid);
        assert_eq!(handle_process_execv_syscall(exec), SyscallResult::Continue(0));

        let frame = runtime.task_frame(fork.child_tid).unwrap();
        assert_eq!(frame.sepc, 0x4000_0000);
        assert_eq!(frame.sp(), report.initial_sp);
    }

    #[test]
    fn execv_syscall_surface_decodes_and_applies_pipeline() {
        const BASE: usize = 0x7000_0000;
        const PATH: usize = BASE;
        const ARG0: usize = BASE + 0x40;
        const ARGV: usize = BASE + 0x100;
        let mut user_mem = [0u8; 512];
        write_cstr_at(&mut user_mem, PATH - BASE, b"/sbin/init");
        write_cstr_at(&mut user_mem, ARG0 - BASE, b"/sbin/init");
        write_usize_at(&mut user_mem, ARGV - BASE, ARG0);
        write_usize_at(&mut user_mem, ARGV - BASE + core::mem::size_of::<usize>(), 0);

        let decoded = decode_syscall(SYS_EXECVE, PATH, ARGV, 0).unwrap();
        assert_eq!(
            decoded,
            Syscall::Execv {
                path_ptr: PATH,
                argv_ptr: ARGV,
                envp_ptr: 0,
            }
        );

        let mut user_vm = VmSpace::<1>::new();
        user_vm.map_anon(BASE, BASE + user_mem.len(), PTE_R).unwrap();
        let user = CheckedUserSlice::new(&user_mem, BASE, &user_vm);

        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut scratch = [0u8; 512];
        let mut stack = [0u8; 256];

        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let parent_pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let mut parent_frame = TrapFrame::default();
        parent_frame.sepc = 0x4000_0000;
        parent_frame.set_sp(0x4010_0000);
        runtime.add_runnable_task(1, parent_pid, parent_frame).unwrap();
        runtime.start_first_task();

        let mut child_frame = TrapFrame::default();
        child_frame.sepc = 0x4000_1000;
        child_frame.set_sp(0x4010_1000);
        let fork = runtime
            .fork_current_process(2, child_frame, VmSpace::<2>::new())
            .unwrap();

        let Syscall::Execv { path_ptr, argv_ptr, .. } = decoded else {
            panic!("expected execv syscall");
        };
        let result = handle_process_execv_checked_user_syscall::<1, 4, 64, 4, 2, 2, 5>(
            &mut runtime,
            fork.child_tid,
            &user,
            path_ptr,
            argv_ptr,
            &vfs,
            &mut scratch,
            &mut stack,
            0x4010_0000,
            VmSpace::<2>::new(),
        );

        assert_eq!(result, SyscallResult::Continue(0));
        let frame = runtime.task_frame(fork.child_tid).unwrap();
        assert_eq!(frame.sepc, 0x4000_0000);
        assert!(frame.sp() < 0x4010_0000);
    }

    #[test]
    fn current_process_execv_syscall_uses_runtime_current_tid() {
        const BASE: usize = 0x7000_0000;
        const PATH: usize = BASE;
        const ARG0: usize = BASE + 0x40;
        const ARGV: usize = BASE + 0x100;
        let mut user_mem = [0u8; 512];
        write_cstr_at(&mut user_mem, PATH - BASE, b"/sbin/init");
        write_cstr_at(&mut user_mem, ARG0 - BASE, b"/sbin/init");
        write_usize_at(&mut user_mem, ARGV - BASE, ARG0);
        write_usize_at(&mut user_mem, ARGV - BASE + core::mem::size_of::<usize>(), 0);

        let decoded = decode_syscall(SYS_EXECVE, PATH, ARGV, 0).unwrap();
        let Syscall::Execv { path_ptr, argv_ptr, .. } = decoded else {
            panic!("expected execv syscall");
        };

        let mut user_vm = VmSpace::<1>::new();
        user_vm.map_anon(BASE, BASE + user_mem.len(), PTE_R).unwrap();
        let user = CheckedUserSlice::new(&user_mem, BASE, &user_vm);

        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut scratch = [0u8; 512];
        let mut stack = [0u8; 256];

        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let mut frame = TrapFrame::default();
        frame.sepc = 0x4000_1000;
        frame.set_sp(0x4010_1000);
        runtime.add_runnable_task(1, pid, frame).unwrap();
        runtime.start_first_task();
        assert_eq!(runtime.current_tid(), Some(1));

        let result = handle_current_process_execv_checked_user_syscall::<1, 4, 64, 4, 2, 2, 5>(
            &mut runtime,
            &user,
            path_ptr,
            argv_ptr,
            &vfs,
            &mut scratch,
            &mut stack,
            0x4010_0000,
            VmSpace::<2>::new(),
        );

        assert_eq!(result, SyscallResult::Continue(0));
        assert_eq!(runtime.current_process_id(), Ok(pid));
        let frame = runtime.task_frame(1).unwrap();
        assert_eq!(frame.sepc, 0x4000_0000);
        assert!(frame.sp() < 0x4010_0000);
    }

    #[test]
    fn execv_dispatch_helper_routes_decoded_execve_to_current_runtime() {
        const BASE: usize = 0x7000_0000;
        const PATH: usize = BASE;
        const ARG0: usize = BASE + 0x40;
        const ARGV: usize = BASE + 0x100;
        let mut user_mem = [0u8; 512];
        write_cstr_at(&mut user_mem, PATH - BASE, b"/sbin/init");
        write_cstr_at(&mut user_mem, ARG0 - BASE, b"/sbin/init");
        write_usize_at(&mut user_mem, ARGV - BASE, ARG0);
        write_usize_at(&mut user_mem, ARGV - BASE + core::mem::size_of::<usize>(), 0);

        let syscall = decode_syscall(SYS_EXECVE, PATH, ARGV, 0).unwrap();
        let mut user_vm = VmSpace::<1>::new();
        user_vm.map_anon(BASE, BASE + user_mem.len(), PTE_R).unwrap();
        let user = CheckedUserSlice::new(&user_mem, BASE, &user_vm);

        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut scratch = [0u8; 512];
        let mut stack = [0u8; 256];

        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let mut frame = TrapFrame::default();
        frame.sepc = 0x4000_1000;
        frame.set_sp(0x4010_1000);
        runtime.add_runnable_task(1, pid, frame).unwrap();
        runtime.start_first_task();

        let result = dispatch_current_execv_syscall::<1, 4, 64, 4, 2, 2, 5>(
            syscall,
            CurrentExecvDispatchContext {
                runtime: &mut runtime,
                user: &user,
                vfs: &vfs,
                scratch: &mut scratch,
                stack_bytes: &mut stack,
                stack_top_va: 0x4010_0000,
                new_vm: VmSpace::<2>::new(),
            },
        );

        assert_eq!(result, SyscallResult::Continue(0));
        assert_eq!(runtime.current_process_id(), Ok(pid));
        let frame = runtime.task_frame(1).unwrap();
        assert_eq!(frame.sepc, 0x4000_0000);
        assert!(frame.sp() < 0x4010_0000);
    }

    #[test]
    fn execv_runtime_dispatch_uses_current_process_vm_for_copyin() {
        const BASE: usize = 0x7000_0000;
        const PATH: usize = BASE;
        const ARG0: usize = BASE + 0x40;
        const ARGV: usize = BASE + 0x100;
        let mut user_mem = [0u8; 512];
        write_cstr_at(&mut user_mem, PATH - BASE, b"/sbin/init");
        write_cstr_at(&mut user_mem, ARG0 - BASE, b"/sbin/init");
        write_usize_at(&mut user_mem, ARGV - BASE, ARG0);
        write_usize_at(&mut user_mem, ARGV - BASE + core::mem::size_of::<usize>(), 0);

        let syscall = decode_syscall(SYS_EXECVE, PATH, ARGV, 0).unwrap();
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut scratch = [0u8; 512];
        let mut stack = [0u8; 256];

        let mut process_vm = VmSpace::<2>::new();
        process_vm
            .map_anon(BASE, BASE + user_mem.len(), PTE_R)
            .unwrap();
        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let pid = runtime.create_process(None, process_vm).unwrap();
        let mut frame = TrapFrame::default();
        frame.sepc = 0x4000_1000;
        frame.set_sp(0x4010_1000);
        runtime.add_runnable_task(1, pid, frame).unwrap();
        runtime.start_first_task();

        let result = dispatch_current_execv_syscall_from_runtime::<4, 64, 4, 2, 2, 5>(
            syscall,
            CurrentExecvRuntimeContext {
                runtime: &mut runtime,
                user_bytes: &user_mem,
                user_base: BASE,
                vfs: &vfs,
                scratch: &mut scratch,
                stack_bytes: &mut stack,
                stack_top_va: 0x4010_0000,
                new_vm: VmSpace::<2>::new(),
            },
        );

        assert_eq!(result, SyscallResult::Continue(0));
        assert_eq!(runtime.current_process_id(), Ok(pid));
        let frame = runtime.task_frame(1).unwrap();
        assert_eq!(frame.sepc, 0x4000_0000);
        assert!(frame.sp() < 0x4010_0000);
    }

    #[test]
    fn execv_frame_dispatch_decodes_register_arguments() {
        const BASE: usize = 0x7000_0000;
        const PATH: usize = BASE;
        const ARG0: usize = BASE + 0x40;
        const ARGV: usize = BASE + 0x100;
        let mut user_mem = [0u8; 512];
        write_cstr_at(&mut user_mem, PATH - BASE, b"/sbin/init");
        write_cstr_at(&mut user_mem, ARG0 - BASE, b"/sbin/init");
        write_usize_at(&mut user_mem, ARGV - BASE, ARG0);
        write_usize_at(&mut user_mem, ARGV - BASE + core::mem::size_of::<usize>(), 0);

        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut scratch = [0u8; 512];
        let mut stack = [0u8; 256];

        let mut process_vm = VmSpace::<2>::new();
        process_vm
            .map_anon(BASE, BASE + user_mem.len(), PTE_R)
            .unwrap();
        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let pid = runtime.create_process(None, process_vm).unwrap();
        let mut task_frame = TrapFrame::default();
        task_frame.sepc = 0x4000_1000;
        task_frame.set_sp(0x4010_1000);
        runtime.add_runnable_task(1, pid, task_frame).unwrap();
        runtime.start_first_task();

        let mut syscall_frame = TrapFrame::default();
        syscall_frame.set_a7(SYS_EXECVE);
        syscall_frame.set_a0(PATH);
        syscall_frame.set_a1(ARGV);
        syscall_frame.set_a2(0);

        let result = dispatch_current_execv_frame_from_runtime::<4, 64, 4, 2, 2, 5>(
            &syscall_frame,
            CurrentExecvRuntimeContext {
                runtime: &mut runtime,
                user_bytes: &user_mem,
                user_base: BASE,
                vfs: &vfs,
                scratch: &mut scratch,
                stack_bytes: &mut stack,
                stack_top_va: 0x4010_0000,
                new_vm: VmSpace::<2>::new(),
            },
        );

        assert_eq!(result, SyscallResult::Continue(0));
        let frame = runtime.task_frame(1).unwrap();
        assert_eq!(frame.sepc, 0x4000_0000);
        assert!(frame.sp() < 0x4010_0000);
    }

    #[test]
    fn execv_frame_writeback_updates_live_trap_frame() {
        const BASE: usize = 0x7000_0000;
        const PATH: usize = BASE;
        const ARG0: usize = BASE + 0x40;
        const ARGV: usize = BASE + 0x100;
        let mut user_mem = [0u8; 512];
        write_cstr_at(&mut user_mem, PATH - BASE, b"/sbin/init");
        write_cstr_at(&mut user_mem, ARG0 - BASE, b"/sbin/init");
        write_usize_at(&mut user_mem, ARGV - BASE, ARG0);
        write_usize_at(&mut user_mem, ARGV - BASE + core::mem::size_of::<usize>(), 0);

        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut scratch = [0u8; 512];
        let mut stack = [0u8; 256];

        let mut process_vm = VmSpace::<2>::new();
        process_vm
            .map_anon(BASE, BASE + user_mem.len(), PTE_R)
            .unwrap();
        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let pid = runtime.create_process(None, process_vm).unwrap();
        let mut task_frame = TrapFrame::default();
        task_frame.sepc = 0x4000_1000;
        task_frame.set_sp(0x4010_1000);
        runtime.add_runnable_task(1, pid, task_frame).unwrap();
        runtime.start_first_task();

        let mut live_frame = TrapFrame::default();
        live_frame.sepc = 0x4000_2000;
        live_frame.set_a7(SYS_EXECVE);
        live_frame.set_a0(PATH);
        live_frame.set_a1(ARGV);
        live_frame.set_a2(0);

        let outcome = handle_current_execv_frame_from_runtime::<4, 64, 4, 2, 2, 5>(
            &mut live_frame,
            CurrentExecvRuntimeContext {
                runtime: &mut runtime,
                user_bytes: &user_mem,
                user_base: BASE,
                vfs: &vfs,
                scratch: &mut scratch,
                stack_bytes: &mut stack,
                stack_top_va: 0x4010_0000,
                new_vm: VmSpace::<2>::new(),
            },
        );

        assert_eq!(outcome, TrapOutcome::Resume);
        assert_eq!(live_frame.a0(), 0);
        assert_eq!(live_frame.sepc, 0x4000_2004);
        let exec_frame = runtime.task_frame(1).unwrap();
        assert_eq!(exec_frame.sepc, 0x4000_0000);
        assert!(exec_frame.sp() < 0x4010_0000);
    }

    #[test]
    fn execv_trap_dispatch_routes_ecall_to_runtime_helper() {
        const BASE: usize = 0x7000_0000;
        const PATH: usize = BASE;
        const ARG0: usize = BASE + 0x40;
        const ARGV: usize = BASE + 0x100;
        let mut user_mem = [0u8; 512];
        write_cstr_at(&mut user_mem, PATH - BASE, b"/sbin/init");
        write_cstr_at(&mut user_mem, ARG0 - BASE, b"/sbin/init");
        write_usize_at(&mut user_mem, ARGV - BASE, ARG0);
        write_usize_at(&mut user_mem, ARGV - BASE + core::mem::size_of::<usize>(), 0);

        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut scratch = [0u8; 512];
        let mut stack = [0u8; 256];

        let mut process_vm = VmSpace::<2>::new();
        process_vm
            .map_anon(BASE, BASE + user_mem.len(), PTE_R)
            .unwrap();
        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let pid = runtime.create_process(None, process_vm).unwrap();
        let mut task_frame = TrapFrame::default();
        task_frame.sepc = 0x4000_1000;
        task_frame.set_sp(0x4010_1000);
        runtime.add_runnable_task(1, pid, task_frame).unwrap();
        runtime.start_first_task();

        let mut live_frame = TrapFrame::default();
        live_frame.sepc = 0x4000_3000;
        live_frame.set_a7(SYS_EXECVE);
        live_frame.set_a0(PATH);
        live_frame.set_a1(ARGV);
        live_frame.set_a2(0);

        let outcome = handle_trap_with_current_execv_runtime::<4, 64, 4, 2, 2, 5>(
            &mut live_frame,
            SCAUSE_USER_ECALL,
            0,
            CurrentExecvRuntimeContext {
                runtime: &mut runtime,
                user_bytes: &user_mem,
                user_base: BASE,
                vfs: &vfs,
                scratch: &mut scratch,
                stack_bytes: &mut stack,
                stack_top_va: 0x4010_0000,
                new_vm: VmSpace::<2>::new(),
            },
        );

        assert_eq!(outcome, TrapOutcome::Resume);
        assert_eq!(live_frame.a0(), 0);
        assert_eq!(live_frame.sepc, 0x4000_3004);
        let exec_frame = runtime.task_frame(1).unwrap();
        assert_eq!(exec_frame.sepc, 0x4000_0000);
        assert!(exec_frame.sp() < 0x4010_0000);
    }

    #[test]
    fn execv_trap_resources_build_current_runtime_context() {
        const BASE: usize = 0x7000_0000;
        const PATH: usize = BASE;
        const ARG0: usize = BASE + 0x40;
        const ARGV: usize = BASE + 0x100;
        let mut user_mem = [0u8; 512];
        write_cstr_at(&mut user_mem, PATH - BASE, b"/sbin/init");
        write_cstr_at(&mut user_mem, ARG0 - BASE, b"/sbin/init");
        write_usize_at(&mut user_mem, ARGV - BASE, ARG0);
        write_usize_at(&mut user_mem, ARGV - BASE + core::mem::size_of::<usize>(), 0);

        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut process_vm = VmSpace::<2>::new();
        process_vm
            .map_anon(BASE, BASE + user_mem.len(), PTE_R)
            .unwrap();
        let mut runtime =
            ProcessTrapRuntime::<4, 2, 2, 5>::new_with_execv_stack_top(0x4010_0000);
        assert_eq!(runtime.execv_stack_top_va(), 0x4010_0000);
        let execv_plan = runtime.execv_dispatch_plan::<512, 256>();
        assert_eq!(execv_plan.stack_top_va, 0x4010_0000);
        assert_eq!(execv_plan.max_areas(), 2);
        let mut execv_dispatch = runtime.prepare_execv_dispatch::<512, 256>(&vfs);

        let pid = runtime.create_process(None, process_vm).unwrap();
        let mut task_frame = TrapFrame::default();
        task_frame.sepc = 0x4000_1000;
        task_frame.set_sp(0x4010_1000);
        runtime.add_runnable_task(1, pid, task_frame).unwrap();
        runtime.start_first_task();

        let mut live_frame = TrapFrame::default();
        live_frame.sepc = 0x4000_4000;
        live_frame.set_a7(SYS_EXECVE);
        live_frame.set_a0(PATH);
        live_frame.set_a1(ARGV);
        live_frame.set_a2(0);

        let outcome = execv_dispatch.dispatch::<4, 64, 4, 2, 5>(
            &mut runtime,
            &mut live_frame,
            SCAUSE_USER_ECALL,
            0,
            &user_mem,
            BASE,
            VmSpace::<2>::new(),
        );

        assert_eq!(outcome, TrapOutcome::Resume);
        assert_eq!(live_frame.a0(), 0);
        assert_eq!(live_frame.sepc, 0x4000_4004);
        let exec_frame = runtime.task_frame(1).unwrap();
        assert_eq!(exec_frame.sepc, 0x4000_0000);
        assert!(exec_frame.sp() < 0x4010_0000);
    }

    #[test]
    fn wait4_syscall_decodes_status_pointer_and_options() {
        assert_eq!(
            decode_syscall(SYS_WAIT4, usize::MAX, 0x4000, 0),
            Some(Syscall::Wait4 {
                pid: -1,
                status_ptr: 0x4000,
                options: 0,
            })
        );
    }

    #[test]
    fn wait4_syscall_copies_exit_status_to_user_memory() {
        let report = ProcessWaitReapRuntime {
            parent_pid: 1,
            child_pid: 2,
            exit_code: 7,
            tasks_reaped: 2,
            scheduler_tasks_reaped: 2,
        };
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0x6000, 0x6008, PTE_W).unwrap();
        let mut bytes = [0u8; 8];
        let mut user = CheckedUserSliceMut::new(&mut bytes, 0x6000, &vm);

        assert_eq!(
            handle_process_wait4_checked_user_syscall(&mut user, 0x6000, report),
            SyscallResult::Continue(2)
        );
        assert_eq!(&bytes[..4], &7i32.to_ne_bytes());
    }

    #[test]
    fn read_syscall_copies_device_bytes_out_to_user_buffer() {
        let dev: &'static MockCharDevice =
            std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"input")));
        let mut runtime = ProcessIoRuntime::<2, 2, 2, 8>::new();
        let pid = runtime
            .create_process_with_stdio(None, VmSpace::<2>::new(), dev)
            .unwrap();
        let task = TaskControlBlock::new(1, pid, TrapFrame::default());
        let current = runtime.current_for_task(&task).unwrap();
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0x4000, 0x4010, PTE_W).unwrap();
        let mut user_bytes = [0u8; 16];
        let mut user = CheckedUserSliceMut::new(&mut user_bytes, 0x4000, &vm);
        let mut scratch = [0u8; 8];

        assert_eq!(
            handle_current_process_read_checked_user_syscall(
                current,
                &mut user,
                0,
                0x4000,
                5,
                &mut scratch,
            ),
            SyscallResult::Continue(5)
        );
        assert_eq!(&user_bytes[..5], b"input");
    }

    #[test]
    fn read_syscall_reports_fd_and_copyout_errors() {
        let dev: &'static MockCharDevice =
            std::boxed::Box::leak(std::boxed::Box::new(MockCharDevice::new(b"input")));
        let mut runtime = ProcessIoRuntime::<2, 2, 2, 8>::new();
        let pid = runtime
            .create_process_with_stdio(None, VmSpace::<2>::new(), dev)
            .unwrap();
        let task = TaskControlBlock::new(1, pid, TrapFrame::default());
        let current = runtime.current_for_task(&task).unwrap();
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0x5000, 0x5010, PTE_R).unwrap();
        let mut user_bytes = [0u8; 16];
        let mut user = CheckedUserSliceMut::new(&mut user_bytes, 0x5000, &vm);
        let mut scratch = [0u8; 8];

        assert_eq!(
            handle_current_process_read_checked_user_syscall(
                current,
                &mut user,
                7,
                0x5000,
                1,
                &mut scratch,
            ),
            SyscallResult::Continue(-EBADF)
        );
        assert_eq!(
            handle_current_process_read_checked_user_syscall(
                current,
                &mut user,
                0,
                0x5000,
                1,
                &mut scratch,
            ),
            SyscallResult::Continue(-EFAULT)
        );
    }

    #[test]
    fn syscall_decodes_read_number() {
        assert_eq!(
            decode_syscall(SYS_READ, 3, 0x1000, 4),
            Some(Syscall::Read { fd: 3, ptr: 0x1000, len: 4 })
        );
    }


    #[test]
    fn buffered_console_device_reads_seed_bytes() {
        let dev = console::BufferedConsoleDevice::new("test-console", b"abc");
        let mut out = [0u8; 2];

        assert_eq!(dev.read(&mut out), Ok(2));
        assert_eq!(&out, b"ab");
        assert_eq!(dev.read(&mut out), Ok(1));
        assert_eq!(out[0], b'c');
        assert_eq!(dev.read(&mut out), Err(CharError::WouldBlock));
        dev.reset();
        assert_eq!(dev.read(&mut out), Ok(2));
    }

    #[test]
    fn read_syscall_copies_buffered_console_bytes() {
        let dev: &'static console::BufferedConsoleDevice =
            std::boxed::Box::leak(std::boxed::Box::new(console::BufferedConsoleDevice::new(
                "test-console",
                b"runtime",
            )));
        let mut runtime = ProcessIoRuntime::<2, 2, 2, 8>::new();
        let pid = runtime
            .create_process_with_stdio(None, VmSpace::<2>::new(), dev)
            .unwrap();
        let task = TaskControlBlock::new(1, pid, TrapFrame::default());
        let current = runtime.current_for_task(&task).unwrap();
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0x6000, 0x6020, PTE_W).unwrap();
        let mut user_bytes = [0u8; 16];
        let mut user = CheckedUserSliceMut::new(&mut user_bytes, 0x6000, &vm);
        let mut scratch = [0u8; 8];

        assert_eq!(
            handle_current_process_read_checked_user_syscall(
                current,
                &mut user,
                0,
                0x6000,
                7,
                &mut scratch,
            ),
            SyscallResult::Continue(7)
        );
        assert_eq!(&user_bytes[..7], b"runtime");
    }


    #[test]
    fn wouldblock_maps_to_eagain_for_nonblocking_read() {
        let dev: &'static console::BufferedConsoleDevice =
            std::boxed::Box::leak(std::boxed::Box::new(console::BufferedConsoleDevice::new(
                "test-console",
                b"xy",
            )));
        let mut runtime = ProcessIoRuntime::<2, 2, 2, 8>::new();
        let pid = runtime
            .create_process_with_stdio(None, VmSpace::<2>::new(), dev)
            .unwrap();
        let task = TaskControlBlock::new(1, pid, TrapFrame::default());
        let current = runtime.current_for_task(&task).unwrap();
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0x7000, 0x7020, PTE_W).unwrap();
        let mut user_bytes = [0u8; 16];
        let mut user = CheckedUserSliceMut::new(&mut user_bytes, 0x7000, &vm);
        let mut scratch = [0u8; 8];

        assert_eq!(
            handle_current_process_read_checked_user_syscall(
                current,
                &mut user,
                0,
                0x7000,
                2,
                &mut scratch,
            ),
            SyscallResult::Continue(2)
        );
        assert_eq!(
            handle_current_process_read_checked_user_syscall(
                current,
                &mut user,
                0,
                0x7002,
                1,
                &mut scratch,
            ),
            SyscallResult::Continue(-EAGAIN)
        );
    }

    #[test]
    fn blocking_read_wouldblock_parks_task_on_wait_queue() {
        let dev: &'static console::BufferedConsoleDevice =
            std::boxed::Box::leak(std::boxed::Box::new(console::BufferedConsoleDevice::new(
                "test-console",
                b"z",
            )));
        let mut runtime = ProcessIoRuntime::<2, 2, 2, 8>::new();
        let pid = runtime
            .create_process_with_stdio(None, VmSpace::<2>::new(), dev)
            .unwrap();
        let task = TaskControlBlock::new(7, pid, TrapFrame::default());
        let current = runtime.current_for_task(&task).unwrap();
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0x8000, 0x8020, PTE_W).unwrap();
        let mut user_bytes = [0u8; 16];
        let mut user = CheckedUserSliceMut::new(&mut user_bytes, 0x8000, &vm);
        let mut scratch = [0u8; 8];
        let mut scheduler = Scheduler::<2>::new();
        scheduler.add_runnable(task.tid, TrapFrame::default()).unwrap();
        let _ = scheduler.on_tick();
        let mut waiters = WaitQueue::<2>::new();

        assert_eq!(
            handle_current_process_blocking_read_checked_user_syscall(
                current,
                task.tid,
                &mut user,
                0,
                0x8000,
                1,
                &mut scratch,
                &mut scheduler,
                &mut waiters,
            ),
            BlockingReadResult::Ready(SyscallResult::Continue(1))
        );
        assert_eq!(
            handle_current_process_blocking_read_checked_user_syscall(
                current,
                task.tid,
                &mut user,
                0,
                0x8001,
                1,
                &mut scratch,
                &mut scheduler,
                &mut waiters,
            ),
            BlockingReadResult::Blocked
        );
        assert_eq!(scheduler.task_state(task.tid), Some(TaskState::Blocked));
        assert!(waiters.contains(task.tid));
        assert_eq!(waiters.wake_one(&mut scheduler), Some(task.tid));
        assert_eq!(scheduler.task_state(task.tid), Some(TaskState::Runnable));
    }

    #[test]
    fn serial_rx_event_wakes_blocked_reader() {
        let mut scheduler = Scheduler::<2>::new();
        scheduler.add_runnable(7, TrapFrame::default()).unwrap();
        let _ = scheduler.on_tick();
        scheduler.set_state(7, TaskState::Blocked);
        let mut waiters = WaitQueue::<2>::new();
        waiters.enqueue(7).unwrap();

        assert_eq!(
            wake_reader_on_rx_event(SerialRxEvent::new(0), &mut waiters, &mut scheduler),
            None
        );
        assert_eq!(scheduler.task_state(7), Some(TaskState::Blocked));
        assert_eq!(
            wake_reader_on_rx_event(SerialRxEvent::new(1), &mut waiters, &mut scheduler),
            Some(7)
        );
        assert_eq!(scheduler.task_state(7), Some(TaskState::Runnable));
    }

    #[test]
    fn blocked_read_result_requests_scheduler_handoff() {
        let mut frame = TrapFrame::default();
        frame.sepc = 0x8020_0000;
        frame.set_a0(0x55);

        assert_eq!(
            handle_blocking_syscall_result(&mut frame, BlockingReadResult::Blocked),
            TrapOutcome::Schedule
        );
        assert_eq!(frame.sepc, 0x8020_0000);
        assert_eq!(frame.a0(), 0x55);

        assert_eq!(
            handle_blocking_syscall_result(
                &mut frame,
                BlockingReadResult::Ready(SyscallResult::Continue(3)),
            ),
            TrapOutcome::Resume
        );
        assert_eq!(frame.sepc, 0x8020_0004);
        assert_eq!(frame.a0(), 3);
    }

    #[test]
    fn plain_syscall_result_still_advances_trap_frame() {
        let mut frame = TrapFrame::default();
        frame.sepc = 0x1000;

        assert_eq!(
            handle_syscall_result(&mut frame, SyscallResult::Continue(-EAGAIN)),
            TrapOutcome::Resume
        );
        assert_eq!(frame.sepc, 0x1004);
        assert_eq!(frame.a0() as isize, -EAGAIN);
    }

    #[test]
    fn scheduler_handoff_saves_blocked_frame_and_selects_next_task() {
        let mut scheduler = Scheduler::<3>::new();
        scheduler.add_runnable(1, TrapFrame::default()).unwrap();
        scheduler.add_runnable(2, TrapFrame::default()).unwrap();
        let _ = scheduler.on_tick();
        assert_eq!(scheduler.current_tid(), Some(1));
        scheduler.set_state(1, TaskState::Blocked);

        let mut blocked_frame = TrapFrame::default();
        blocked_frame.sepc = 0x9000;
        blocked_frame.set_a0(0x1234);

        assert_eq!(
            schedule_after_blocked_syscall(&blocked_frame, &mut scheduler),
            Some(ScheduleDecision::Switched {
                prev_tid: 1,
                next_tid: 2,
            })
        );
        assert_eq!(scheduler.current_tid(), Some(2));
        assert_eq!(scheduler.task_state(1), Some(TaskState::Blocked));
        assert_eq!(scheduler.task_state(2), Some(TaskState::Running));
        let saved = scheduler.task_frame(1).unwrap();
        assert_eq!(saved.sepc, 0x9000);
        assert_eq!(saved.a0(), 0x1234);
    }


    #[test]
    fn scheduler_handoff_enters_idle_when_no_runnable_task_exists() {
        let mut scheduler = Scheduler::<2>::new();
        scheduler.add_runnable(1, TrapFrame::default()).unwrap();
        let _ = scheduler.on_tick();
        scheduler.set_state(1, TaskState::Blocked);

        let mut blocked_frame = TrapFrame::default();
        blocked_frame.sepc = 0xa000;

        assert_eq!(
            schedule_after_blocked_syscall(&blocked_frame, &mut scheduler),
            Some(ScheduleDecision::Idle { blocked_tid: 1 })
        );
        assert_eq!(scheduler.current_tid(), None);
        assert_eq!(scheduler.task_state(1), Some(TaskState::Blocked));
        assert_eq!(scheduler.task_frame(1).unwrap().sepc, 0xa000);
    }


    #[test]
    fn idle_decision_maps_to_wait_for_interrupt_action() {
        assert_eq!(
            idle_action_for_schedule_decision(ScheduleDecision::Idle { blocked_tid: 7 }),
            Some(IdleAction::WaitForInterrupt { blocked_tid: 7 })
        );
        assert_eq!(
            idle_action_for_schedule_decision(ScheduleDecision::Switched {
                prev_tid: 1,
                next_tid: 2,
            }),
            None
        );
    }


    #[test]
    fn switched_decision_restores_next_task_frame_into_dispatch_frame() {
        let mut next_frame = TrapFrame::default();
        next_frame.sepc = 0xb000;
        next_frame.set_a0(0x44);
        next_frame.set_sp(0xc000);

        let mut scheduler = Scheduler::<2>::new();
        scheduler.add_runnable(1, TrapFrame::default()).unwrap();
        scheduler.add_runnable(2, next_frame).unwrap();
        let _ = scheduler.on_tick();
        scheduler.set_state(1, TaskState::Blocked);
        let mut blocked_frame = TrapFrame::default();
        blocked_frame.sepc = 0xa000;
        let decision = schedule_after_blocked_syscall(&blocked_frame, &mut scheduler).unwrap();

        let mut dispatch_frame = blocked_frame;
        assert_eq!(
            apply_schedule_decision(&mut dispatch_frame, &scheduler, decision),
            Some(ScheduleDecision::Switched {
                prev_tid: 1,
                next_tid: 2,
            })
        );
        assert_eq!(dispatch_frame.sepc, 0xb000);
        assert_eq!(dispatch_frame.a0(), 0x44);
        assert_eq!(dispatch_frame.sp(), 0xc000);
    }


    #[test]
    fn trap_schedule_outcome_dispatches_to_next_task_frame() {
        let mut next_frame = TrapFrame::default();
        next_frame.sepc = 0xc000;
        next_frame.set_a0(0x54);
        next_frame.set_sp(0xd000);

        let mut scheduler = Scheduler::<2>::new();
        scheduler.add_runnable(1, TrapFrame::default()).unwrap();
        scheduler.add_runnable(2, next_frame).unwrap();
        let _ = scheduler.on_tick();
        scheduler.set_state(1, TaskState::Blocked);

        let mut dispatch_frame = TrapFrame::default();
        dispatch_frame.sepc = 0xb000;
        assert_eq!(
            dispatch_trap_outcome(&mut dispatch_frame, &mut scheduler, TrapOutcome::Schedule),
            TrapDispatchResult::Switched(ScheduleDecision::Switched {
                prev_tid: 1,
                next_tid: 2,
            })
        );
        assert_eq!(scheduler.current_tid(), Some(2));
        assert_eq!(scheduler.task_frame(1).unwrap().sepc, 0xb000);
        assert_eq!(dispatch_frame.sepc, 0xc000);
        assert_eq!(dispatch_frame.a0(), 0x54);
        assert_eq!(dispatch_frame.sp(), 0xd000);
    }

    #[test]
    fn trap_schedule_outcome_dispatches_to_idle_action() {
        let mut scheduler = Scheduler::<1>::new();
        scheduler.add_runnable(7, TrapFrame::default()).unwrap();
        let _ = scheduler.on_tick();
        scheduler.set_state(7, TaskState::Blocked);

        let mut dispatch_frame = TrapFrame::default();
        dispatch_frame.sepc = 0xe000;
        assert_eq!(
            dispatch_trap_outcome(&mut dispatch_frame, &mut scheduler, TrapOutcome::Schedule),
            TrapDispatchResult::Idle(IdleAction::WaitForInterrupt { blocked_tid: 7 })
        );
        assert_eq!(scheduler.current_tid(), None);
        assert_eq!(scheduler.task_frame(7).unwrap().sepc, 0xe000);
    }


    #[test]
    fn trap_runtime_owns_scheduler_state_for_schedule_dispatch() {
        let mut next_frame = TrapFrame::default();
        next_frame.sepc = 0xf000;
        next_frame.set_a0(0x55);
        next_frame.set_sp(0x10000);

        let mut runtime = TrapRuntime::<2>::new();
        runtime.add_runnable(1, TrapFrame::default()).unwrap();
        runtime.add_runnable(2, next_frame).unwrap();
        runtime.start_first_task();
        runtime.mark_blocked(1);

        let mut dispatch_frame = TrapFrame::default();
        dispatch_frame.sepc = 0xe000;
        let result = runtime.dispatch_schedule(&mut dispatch_frame);

        assert_eq!(switched_tids(result), Some((1, 2)));
        assert_eq!(runtime.current_tid(), Some(2));
        assert_eq!(runtime.task_frame(1).unwrap().sepc, 0xe000);
        assert_eq!(dispatch_frame.sepc, 0xf000);
        assert_eq!(dispatch_frame.a0(), 0x55);
        assert_eq!(dispatch_frame.sp(), 0x10000);
    }


    #[test]
    fn global_trap_runtime_dispatches_schedule_state() {
        reset_global_trap_runtime();

        let mut next_frame = TrapFrame::default();
        next_frame.sepc = 0x11000;
        next_frame.set_a0(0x56);

        let mut dispatch_frame = TrapFrame::default();
        dispatch_frame.sepc = 0x10000;

        let result = with_global_trap_runtime(|runtime| {
            runtime.add_runnable(1, TrapFrame::default()).unwrap();
            runtime.add_runnable(2, next_frame).unwrap();
            runtime.start_first_task();
            runtime.mark_blocked(1);
            runtime.dispatch_schedule(&mut dispatch_frame)
        });

        assert_eq!(switched_tids(result), Some((1, 2)));
        assert_eq!(
            with_global_trap_runtime(|runtime| runtime.current_tid()),
            Some(2)
        );
        assert_eq!(dispatch_frame.sepc, 0x11000);
        assert_eq!(dispatch_frame.a0(), 0x56);

        reset_global_trap_runtime();
    }


    #[test]
    fn trap_dispatch_uses_global_runtime_for_schedule_outcome() {
        reset_global_trap_runtime();

        let mut next_frame = TrapFrame::default();
        next_frame.sepc = 0x13000;
        next_frame.set_a0(0x57);
        next_frame.set_sp(0x14000);

        with_global_trap_runtime(|runtime| {
            runtime.add_runnable(1, TrapFrame::default()).unwrap();
            runtime.add_runnable(2, next_frame).unwrap();
            runtime.start_first_task();
            runtime.mark_blocked(1);
        });

        let mut dispatch_frame = TrapFrame::default();
        dispatch_frame.sepc = 0x12000;
        let result = dispatch_trap_outcome_with_global_runtime(
            &mut dispatch_frame,
            TrapOutcome::Schedule,
        );

        assert_eq!(switched_tids(result), Some((1, 2)));
        assert_eq!(
            with_global_trap_runtime(|runtime| runtime.current_tid()),
            Some(2)
        );
        assert_eq!(
            with_global_trap_runtime(|runtime| runtime.task_frame(1).unwrap().sepc),
            0x12000
        );
        assert_eq!(dispatch_frame.sepc, 0x13000);
        assert_eq!(dispatch_frame.a0(), 0x57);
        assert_eq!(dispatch_frame.sp(), 0x14000);

        reset_global_trap_runtime();
    }

    #[test]
    fn align_up_keeps_aligned_values() {
        assert_eq!(align_up(0x1000, 0x1000), 0x1000);
    }

    #[test]
    fn align_up_rounds_to_next_boundary() {
        assert_eq!(align_up(0x1001, 0x1000), 0x2000);
    }

    #[test]
    fn kheap_region_end_is_calculated() {
        let region = init_kheap(0x9000_0000, 0x10000);
        assert_eq!(region.end(), 0x9001_0000);
    }



    #[test]
    fn serial_device_is_a_named_character_device() {
        let serial = SerialDevice::new("ttyS0", UART0_BASE);
        assert_eq!(serial.name(), "ttyS0");
        assert_eq!(serial.kind(), DeviceKind::Char);
        assert_eq!(serial.base(), UART0_BASE);
    }

    #[test]
    fn serial_device_read_is_unsupported_for_now() {
        let serial = SerialDevice::new("ttyS0", UART0_BASE);
        let mut out = [0u8; 1];
        assert_eq!(serial.read(&mut out), Err(CharError::Unsupported));
    }

    #[test]
    fn mock_char_device_is_registered_as_char_kind() {
        let dev = MockCharDevice::new(b"");
        assert_eq!(dev.name(), "mocktty0");
        assert_eq!(dev.kind(), DeviceKind::Char);
    }

    #[test]
    fn character_device_write_reports_written_len() {
        let dev = MockCharDevice::new(b"");
        assert_eq!(dev.write(b"hello"), Ok(5));
        assert_eq!(dev.written(), b"hello".to_vec());
    }

    #[test]
    fn character_device_read_drains_available_stream_bytes() {
        let dev = MockCharDevice::new(b"abc");
        let mut out = [0u8; 2];
        assert_eq!(dev.read(&mut out), Ok(2));
        assert_eq!(&out, b"ab");

        let mut tail = [0u8; 2];
        assert_eq!(dev.read(&mut tail), Ok(1));
        assert_eq!(tail[0], b'c');
        assert_eq!(dev.read(&mut tail), Err(CharError::WouldBlock));
    }

    #[test]
    fn character_device_rejects_empty_buffers() {
        let dev = MockCharDevice::new(b"x");
        let mut out = [];
        assert_eq!(dev.read(&mut out), Err(CharError::BadBuffer));
        assert_eq!(dev.write(b""), Err(CharError::BadBuffer));
    }

    #[test]
    fn ramdisk_is_registered_as_block_kind() {
        let storage = [0u8; 16];
        let dev = RamDisk::new("ramblk0", &storage, 8);
        assert_eq!(dev.name(), "ramblk0");
        assert_eq!(dev.kind(), DeviceKind::Block);
        assert_eq!(dev.total_blocks(), 2);
    }

    #[test]
    fn block_read_single_lba() {
        let storage: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let dev = RamDisk::new("ramblk0", &storage, 8);
        let mut out = [0u8; 8];
        dev.read_block(1, &mut out).unwrap();
        assert_eq!(out, [8, 9, 10, 11, 12, 13, 14, 15]);
    }

    #[test]
    fn block_read_rejects_wrong_sized_buffer() {
        let storage = [0u8; 16];
        let dev = RamDisk::new("ramblk0", &storage, 8);
        let mut out = [0u8; 4];
        assert_eq!(dev.read_block(0, &mut out), Err(BlockError::BadBuffer));
    }

    #[test]
    fn block_read_across_boundary() {
        let storage: [u8; 16] = [
            b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N',
            b'O', b'P',
        ];
        let dev = RamDisk::new("ramblk0", &storage, 8);
        let mut out = [0u8; 6];
        read_bytes(&dev, 6, &mut out).unwrap();
        assert_eq!(&out, b"GHIJKL");
    }

    #[test]
    fn block_read_reports_out_of_range() {
        let storage = [0u8; 16];
        let dev = RamDisk::new("ramblk0", &storage, 8);
        let mut out = [0u8; 4];
        assert_eq!(read_bytes(&dev, 14, &mut out), Err(BlockError::OutOfRange));
    }

    #[test]
    fn mbr_partition_wrapper_offsets_reads() {
        let mut storage = [0u8; 1024];
        storage[510] = 0x55;
        storage[511] = 0xaa;
        storage[446 + 8..446 + 12].copy_from_slice(&1u32.to_le_bytes());
        storage[512..520].copy_from_slice(b"PARTDATA");

        let dev = RamDisk::new("disk0", &storage, 512);
        let start = super::block::mbr_first_partition_lba(&dev).unwrap();
        let part = super::block::PartitionBlockDevice::new("disk0p1", &dev, start);
        let mut out = [0u8; 512];
        part.read_block(0, &mut out).unwrap();
        assert_eq!(&out[..8], b"PARTDATA");
    }

    #[test]
    fn ext4_superblock_magic_is_valid() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let fs = Ext4Fs::open(&dev).unwrap();
        assert_eq!(fs.superblock().magic, 0xEF53);
        assert_eq!(fs.superblock().block_size(), 1024);
    }

    #[test]
    fn ext4_inode_by_number_reads_root_directory() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let fs = Ext4Fs::open(&dev).unwrap();
        let root = fs.inode_by_number(2).unwrap();
        assert!(root.is_dir());
        assert_eq!(root.size(), 1024);
    }

    #[test]
    fn ext4_lookup_path_finds_sbin_init() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let fs = Ext4Fs::open(&dev).unwrap();
        let inode = fs.lookup_path("/sbin/init").unwrap();
        assert!(inode.is_regular_file());
        assert_eq!(inode.number(), 13);
        assert_eq!(inode.size(), build_test_init_elf().len());
    }

    #[test]
    fn ext4_read_file_bytes_returns_init_elf() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let fs = Ext4Fs::open(&dev).unwrap();
        let inode = fs.lookup_path("/sbin/init").unwrap();
        let mut out = [0u8; 512];
        let copied = fs.read_file(&inode, &mut out).unwrap();
        assert_eq!(copied, build_test_init_elf().len());
        assert_eq!(&out[..4], b"\x7fELF");
    }

    #[test]
    fn vfs_read_all_returns_init_elf_bytes() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut out = [0u8; 512];
        let copied = vfs.read_all("/sbin/init", &mut out).unwrap();
        assert_eq!(copied, build_test_init_elf().len());
        assert_eq!(&out[..4], b"\x7fELF");
    }

    #[test]
    fn vfs_read_all_fails_on_directory() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut out = [0u8; 512];
        assert_eq!(vfs.read_all("/sbin", &mut out), Err(FsError::NotFile));
    }

    #[test]
    fn elf_loader_rejects_bad_magic() {
        let bytes = [0u8; 64];
        assert_eq!(parse_elf_header(&bytes), Err(ElfError::BadMagic));
    }

    #[test]
    fn elf_loader_collects_pt_load_segment() {
        let elf = build_test_init_elf();
        let image = load_user_elf(&elf, 0x4010_0000).unwrap();
        assert_eq!(image.entry, 0x4000_0000);
        assert_eq!(image.segment_count, 1);
        let seg = image.segments[0].unwrap();
        assert_eq!(seg.vaddr, 0x4000_0000);
        assert!(seg.file_size >= 64);
        assert_eq!(seg.mem_size, seg.file_size);
        assert_eq!(&seg.data[..4], &[0x97, 0x05, 0x00, 0x00]);
        assert!(seg
            .data
            .windows(b"[user-init]".len())
            .any(|w| w == b"[user-init]"));
    }

    #[test]
    fn elf_loader_maps_pflags_to_pte_bits() {
        assert_eq!(pflags_to_pte(5), PTE_U | PTE_R | PTE_X);
        assert_eq!(pflags_to_pte(6), PTE_U | PTE_R | PTE_W);
    }

    #[test]
    fn vfs_init_elf_can_be_parsed_by_loader() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut scratch = [0u8; 512];
        let loaded = load_init_from_vfs(&vfs, "/sbin/init", &mut scratch, 0x4010_0000).unwrap();
        assert_eq!(loaded.entry, 0x4000_0000);
        assert_eq!(loaded.stack_top, 0x4010_0000);
        assert_eq!(loaded.segment_count, 1);
    }

    #[test]
    fn page_frame_allocator_allocates_sequential_frames() {
        let mut pfa = PageFrameAllocator::new(0x8020_0000, 0x8020_3000, 0x1000);
        assert_eq!(pfa.alloc(), Some(0x8020_0000));
        assert_eq!(pfa.alloc(), Some(0x8020_1000));
        assert_eq!(pfa.alloc(), Some(0x8020_2000));
        assert_eq!(pfa.alloc(), None);
    }

    #[test]
    fn syscall_decode_maps_write_and_exit() {
        assert_eq!(
            decode_syscall(SYS_OPENAT, usize::MAX, 0x2000, 1),
            Some(Syscall::OpenAt {
                dirfd: usize::MAX,
                path_ptr: 0x2000,
                flags: 1,
                mode: 0,
            })
        );
        assert_eq!(
            decode_syscall(SYS_WRITE, 1, 0x1000, 5),
            Some(Syscall::Write {
                fd: 1,
                ptr: 0x1000,
                len: 5
            })
        );
        assert_eq!(
            decode_syscall(SYS_EXIT, 7, 0, 0),
            Some(Syscall::Exit { code: 7 })
        );
    }

    #[test]
    fn syscall_handler_returns_exit() {
        assert_eq!(handle_syscall(SYS_EXIT, 3, 0, 0), SyscallResult::Exit(3));
    }

    #[test]
    fn kernel_trap_stack_switch_reserves_frame_space() {
        let switch = prepare_kernel_trap_stack_switch(0x4010_0000, 0x8020_8000, 0x8020_7000)
            .unwrap();

        assert_eq!(
            switch,
            KernelTrapStackSwitch {
                user_sp: 0x4010_0000,
                kernel_frame_sp: 0x8020_8000 - TRAP_FRAME_SIZE,
                frame_size: TRAP_FRAME_SIZE,
            }
        );
        assert_eq!(switch.kernel_frame_sp & 0xf, 0);
    }

    #[test]
    fn kernel_trap_stack_switch_rejects_bad_stack_bounds() {
        assert_eq!(
            prepare_kernel_trap_stack_switch(0x4010_0000, 0x8020_7100, 0x8020_7000),
            Err(KernelTrapStackError::StackTooSmall)
        );
        assert_eq!(
            prepare_kernel_trap_stack_switch(0x4010_0000, 0x8020_8008, 0x8020_7000),
            Err(KernelTrapStackError::MisalignedTop)
        );
    }

    #[test]
    fn trap_entry_scratch_points_to_kernel_frame() {
        let scratch = prepare_trap_entry_scratch(0x4010_0000, 0x8020_8000, 0x8020_7000)
            .unwrap();

        assert_eq!(scratch.user_sp, 0x4010_0000);
        assert_eq!(scratch.kernel_frame_sp, 0x8020_8000 - TRAP_FRAME_SIZE);
        assert_eq!(scratch.sscratch, scratch.kernel_frame_sp);
    }

    #[test]
    fn trap_entry_stack_swap_preserves_user_sp_and_activates_kernel_frame() {
        let scratch = prepare_trap_entry_scratch(0x4010_0000, 0x8020_8000, 0x8020_7000)
            .unwrap();
        let swap = simulate_trap_entry_stack_swap(scratch);

        assert_eq!(swap.active_sp, scratch.kernel_frame_sp);
        assert_eq!(swap.saved_user_sp, scratch.user_sp);
    }

    #[test]
    fn trap_entry_return_restores_scratch_for_next_user_trap() {
        let scratch = prepare_trap_entry_scratch(0x4010_0000, 0x8020_8000, 0x8020_7000)
            .unwrap();
        let ret = simulate_trap_entry_return_scratch(scratch);

        assert_eq!(ret.restored_user_sp, scratch.user_sp);
        assert_eq!(ret.next_sscratch, scratch.kernel_frame_sp);
    }

    #[test]
    fn enter_user_prepares_sscratch_from_user_stack_pointer() {
        let mut frame = TrapFrame::default();
        frame.set_sp(0x4010_0000);

        let scratch = prepare_enter_user_scratch(&frame).unwrap();

        assert_eq!(scratch.user_sp, frame.sp());
        assert_eq!(scratch.kernel_frame_sp, super::trap::kernel_trap_stack_top() - TRAP_FRAME_SIZE);
        assert_eq!(scratch.kernel_frame_sp & 0xf, 0);
        assert_eq!(scratch.sscratch, scratch.kernel_frame_sp);
    }

    #[test]
    fn trap_handler_advances_sepc_on_user_ecall() {
        let mut frame = TrapFrame::default();
        frame.sepc = 0x8020_1000;
        frame.set_a7(SYS_WRITE);
        frame.set_a0(1);
        frame.set_a1(0x9000_0000);
        frame.set_a2(4);

        assert_eq!(
            handle_trap(&mut frame, SCAUSE_USER_ECALL, 0),
            TrapOutcome::Resume
        );
        assert_eq!(frame.sepc, 0x8020_1004);
    }

    #[test]
    fn trap_page_fault_outcome_resumes_on_valid_mapping() {
        let mut vm = VmSpace::<4>::new();
        vm.map_anon(0x4000_0000, 0x4000_1000, PTE_R | PTE_W)
            .unwrap();
        let mut pmm = PageFrameManager::<2>::new(0x9000_0000, 0x9000_2000, 0x1000);

        let outcome =
            handle_user_page_fault_outcome(SCAUSE_STORE_PAGE_FAULT, 0x4000_0010, &vm, &mut pmm);
        assert_eq!(outcome, TrapOutcome::Resume);
    }

    #[test]
    fn trap_page_fault_outcome_exits_on_invalid_access() {
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0x4000_0000, 0x4000_1000, PTE_R).unwrap();
        let mut pmm = PageFrameManager::<2>::new(0x9000_0000, 0x9000_2000, 0x1000);

        let outcome =
            handle_user_page_fault_outcome(SCAUSE_STORE_PAGE_FAULT, 0x4000_0010, &vm, &mut pmm);
        assert_eq!(outcome, TrapOutcome::Exit(-1));
    }

    #[test]
    fn scheduler_round_robin_switches_between_two_tasks() {
        let mut sched = Scheduler::<4>::new();
        sched.add_runnable(1, TrapFrame::default()).unwrap();
        sched.add_runnable(2, TrapFrame::default()).unwrap();

        assert_eq!(sched.on_tick(), None);
        assert_eq!(sched.current_tid(), Some(1));
        assert_eq!(sched.on_tick(), Some((1, 2)));
        assert_eq!(sched.current_tid(), Some(2));
    }

    #[test]
    fn scheduler_skips_blocked_task() {
        let mut sched = Scheduler::<4>::new();
        sched.add_runnable(1, TrapFrame::default()).unwrap();
        sched.add_runnable(2, TrapFrame::default()).unwrap();
        sched.set_state(2, TaskState::Blocked);

        assert_eq!(sched.on_tick(), None);
        assert_eq!(sched.current_tid(), Some(1));
        assert_eq!(sched.on_tick(), None);
        assert_eq!(sched.current_tid(), Some(1));
    }

    #[test]
    fn timer_tick_handler_updates_scheduler_ticks() {
        let mut sched = Scheduler::<2>::new();
        sched.add_runnable(1, TrapFrame::default()).unwrap();
        let mut tf = TrapFrame::default();

        assert_eq!(sched.tick_count(), 0);
        assert_eq!(
            handle_timer_tick_with_scheduler(&mut tf, &mut sched),
            TrapOutcome::Resume
        );
        assert_eq!(sched.tick_count(), 1);
    }

    #[test]
    fn context_switch_saves_prev_and_restores_next() {
        let mut live = TrapFrame::default();
        live.sepc = 0x1000;
        live.set_sp(0x8000_1000);

        let mut prev_slot = TrapFrame::default();
        let mut next_slot = TrapFrame::default();
        next_slot.sepc = 0x2000;
        next_slot.set_sp(0x8000_2000);

        let rec = switch_context(1, 2, &mut live, &mut prev_slot, &next_slot);
        assert_eq!(rec.prev_tid, 1);
        assert_eq!(rec.next_tid, 2);
        assert_eq!(prev_slot.sepc, 0x1000);
        assert_eq!(prev_slot.sp(), 0x8000_1000);
        assert_eq!(live.sepc, 0x2000);
        assert_eq!(live.sp(), 0x8000_2000);
    }

    #[test]
    fn process_table_links_parent_and_child() {
        let mut table = ProcessTable::<4, 4, 4>::new();
        let parent_vm = VmSpace::<4>::new();
        let child_vm = VmSpace::<4>::new();
        let parent_pid = table.create_process(None, parent_vm).unwrap();
        let child_pid = table.create_process(Some(parent_pid), child_vm).unwrap();

        let parent = table.get_mut(parent_pid).unwrap();
        parent.add_child(child_pid).unwrap();
        assert_eq!(parent.children[0], Some(child_pid));
    }

    #[test]
    fn pcb_reap_requires_zombie_state() {
        let mut table = ProcessTable::<2, 2, 4>::new();
        let pid = table.create_process(None, VmSpace::<4>::new()).unwrap();
        let pcb = table.get_mut(pid).unwrap();

        assert_eq!(pcb.reap(), Err(ProcError::NotZombie));
        pcb.mark_zombie(7);
        assert_eq!(pcb.state, ProcState::Zombie);
        assert_eq!(pcb.reap().unwrap(), 7);
        assert_eq!(pcb.state, ProcState::Reaped);
    }

    #[test]
    fn task_table_tracks_owner_and_runtime_ticks() {
        let mut tasks = TaskTable::<4>::new();
        let mut tf = TrapFrame::default();
        tf.sepc = 0x4000_0000;
        let tcb = TaskControlBlock::new(1, 10, tf);
        tasks.add_task(tcb).unwrap();

        let slot = tasks.get_mut(1).unwrap();
        assert_eq!(slot.owner_pid, 10);
        assert_eq!(slot.trapframe.sepc, 0x4000_0000);
        slot.runtime_ticks += 1;
        assert_eq!(slot.runtime_ticks, 1);
    }

    #[test]
    fn process_runtime_demo_runs_fork_exec_wait_lifecycle() {
        let report = run_fork_exec_wait_demo().unwrap();
        assert_eq!(report.parent_pid, 1);
        assert_eq!(report.child_pid, 2);
        assert_eq!(report.child_tid, 1);
        assert_eq!(report.exec_entry, 0x4000_1000);
        assert_eq!(report.exec_stack_top, 0x4010_1000);
        assert_eq!(report.reap_code, 0);
    }

    #[test]
    fn process_exit_runtime_marks_owner_process_and_tasks() {
        let mut processes = ProcessTable::<3, 2, 2>::new();
        let mut tasks = TaskTable::<4>::new();
        let owner_pid = processes
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let other_pid = processes
            .create_process(None, VmSpace::<2>::new())
            .unwrap();

        tasks
            .add_task(TaskControlBlock::new(1, owner_pid, TrapFrame::default()))
            .unwrap();
        tasks
            .add_task(TaskControlBlock::new(2, owner_pid, TrapFrame::default()))
            .unwrap();
        tasks
            .add_task(TaskControlBlock::new(3, other_pid, TrapFrame::default()))
            .unwrap();

        let report = exit_process_from_task(&mut processes, &mut tasks, 1, 5).unwrap();

        assert_eq!(report.owner_pid, owner_pid);
        assert_eq!(report.exit_code, 5);
        assert_eq!(report.tasks_marked, 2);
        assert_eq!(processes.get(owner_pid).unwrap().state, ProcState::Zombie);
        assert_eq!(processes.get(owner_pid).unwrap().exit_code, Some(5));
        assert_eq!(
            tasks.count_owner_tasks_with_state(owner_pid, TaskState::Zombie),
            2
        );
        assert_eq!(tasks.get(3).unwrap().sched_state, TaskState::Runnable);
    }

    #[test]
    fn process_exit_runtime_hands_off_scheduler_to_other_process() {
        let mut processes = ProcessTable::<3, 2, 2>::new();
        let mut tasks = TaskTable::<4>::new();
        let mut scheduler = Scheduler::<4>::new();
        let owner_pid = processes
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let other_pid = processes
            .create_process(None, VmSpace::<2>::new())
            .unwrap();

        for (tid, pid) in [(1, owner_pid), (2, owner_pid), (3, other_pid)] {
            let mut frame = TrapFrame::default();
            frame.sepc = 0x4000_0000 + tid * 0x1000;
            tasks
                .add_task(TaskControlBlock::new(tid, pid, frame))
                .unwrap();
            scheduler.add_runnable(tid, frame).unwrap();
        }
        assert_eq!(scheduler.on_tick(), None);
        assert_eq!(scheduler.current_tid(), Some(1));

        let report = exit_process_from_task_with_scheduler(
            &mut processes,
            &mut tasks,
            &mut scheduler,
            1,
            9,
        )
        .unwrap();

        assert_eq!(report.exit.owner_pid, owner_pid);
        assert_eq!(report.exit.tasks_marked, 2);
        assert_eq!(report.scheduler_tasks_marked, 2);
        assert_eq!(
            report.decision,
            ScheduleDecision::Switched {
                prev_tid: 1,
                next_tid: 3
            }
        );
        assert_eq!(scheduler.task_state(1), Some(TaskState::Zombie));
        assert_eq!(scheduler.task_state(2), Some(TaskState::Zombie));
        assert_eq!(scheduler.task_state(3), Some(TaskState::Running));
        assert_eq!(scheduler.current_tid(), Some(3));
        assert_eq!(processes.get(owner_pid).unwrap().exit_code, Some(9));
    }

    #[test]
    fn trap_exit_outcome_dispatches_process_runtime_and_restores_next_frame() {
        let mut processes = ProcessTable::<3, 2, 2>::new();
        let mut tasks = TaskTable::<4>::new();
        let mut scheduler = Scheduler::<4>::new();
        let owner_pid = processes
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let other_pid = processes
            .create_process(None, VmSpace::<2>::new())
            .unwrap();

        for (tid, pid) in [(1, owner_pid), (2, owner_pid), (3, other_pid)] {
            let mut frame = TrapFrame::default();
            frame.sepc = 0x4000_0000 + tid * 0x1000;
            frame.set_a0(tid);
            frame.set_sp(0x4010_0000 + tid * 0x1000);
            tasks
                .add_task(TaskControlBlock::new(tid, pid, frame))
                .unwrap();
            scheduler.add_runnable(tid, frame).unwrap();
        }
        let _ = scheduler.on_tick();

        let mut live = TrapFrame::default();
        live.sepc = 0xfeed_0000;
        live.set_sp(0xdead_0000);
        let result = dispatch_exit_outcome_with_process_runtime(
            &mut live,
            &mut processes,
            &mut tasks,
            &mut scheduler,
            1,
            11,
        );

        assert_eq!(
            result,
            TrapDispatchResult::Switched(ScheduleDecision::Switched {
                prev_tid: 1,
                next_tid: 3
            })
        );
        assert_eq!(live.sepc, 0x4000_3000);
        assert_eq!(live.a0(), 3);
        assert_eq!(live.sp(), 0x4010_3000);
        assert_eq!(scheduler.task_state(1), Some(TaskState::Zombie));
        assert_eq!(scheduler.task_state(2), Some(TaskState::Zombie));
        assert_eq!(scheduler.task_state(3), Some(TaskState::Running));
        assert_eq!(processes.get(owner_pid).unwrap().exit_code, Some(11));
    }

    #[test]
    fn process_trap_runtime_owns_tables_and_dispatches_exit() {
        let mut runtime = ProcessTrapRuntime::<3, 2, 2, 4>::new();
        let owner_pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let other_pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();

        for (tid, pid) in [(1, owner_pid), (2, owner_pid), (3, other_pid)] {
            let mut frame = TrapFrame::default();
            frame.sepc = 0x4000_0000 + tid * 0x1000;
            frame.set_a0(tid);
            frame.set_sp(0x4010_0000 + tid * 0x1000);
            runtime.add_runnable_task(tid, pid, frame).unwrap();
        }
        runtime.start_first_task();

        let mut live = TrapFrame::default();
        let result = runtime.dispatch_exit(&mut live, 1, 13);

        assert_eq!(
            result,
            TrapDispatchResult::Switched(ScheduleDecision::Switched {
                prev_tid: 1,
                next_tid: 3
            })
        );
        assert_eq!(live.sepc, 0x4000_3000);
        assert_eq!(live.a0(), 3);
        assert_eq!(live.sp(), 0x4010_3000);
        assert_eq!(runtime.task_state(1), Some(TaskState::Zombie));
        assert_eq!(runtime.task_state(2), Some(TaskState::Zombie));
        assert_eq!(runtime.task_state(3), Some(TaskState::Running));
        assert_eq!(runtime.current_tid(), Some(3));
        assert_eq!(runtime.process_state(owner_pid), Ok(ProcState::Zombie));
        assert_eq!(runtime.process_exit_code(owner_pid), Ok(Some(13)));
    }

    #[test]
    fn process_reap_runtime_removes_owner_tasks_and_scheduler_slots() {
        let mut processes = ProcessTable::<3, 2, 2>::new();
        let mut tasks = TaskTable::<4>::new();
        let mut scheduler = Scheduler::<4>::new();
        let owner_pid = processes
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let other_pid = processes
            .create_process(None, VmSpace::<2>::new())
            .unwrap();

        for (tid, pid) in [(1, owner_pid), (2, owner_pid), (3, other_pid)] {
            let frame = TrapFrame::default();
            tasks
                .add_task(TaskControlBlock::new(tid, pid, frame))
                .unwrap();
            scheduler.add_runnable(tid, frame).unwrap();
        }
        let _ = scheduler.on_tick();
        let _ = exit_process_from_task_with_scheduler(
            &mut processes,
            &mut tasks,
            &mut scheduler,
            1,
            21,
        )
        .unwrap();

        let report =
            reap_exited_process(&mut processes, &mut tasks, &mut scheduler, owner_pid).unwrap();

        assert_eq!(report.pid, owner_pid);
        assert_eq!(report.exit_code, 21);
        assert_eq!(report.tasks_reaped, 2);
        assert_eq!(report.scheduler_tasks_reaped, 2);
        assert_eq!(processes.get(owner_pid).unwrap().state, ProcState::Reaped);
        assert!(tasks.get(1).is_err());
        assert!(tasks.get(2).is_err());
        assert_eq!(tasks.get(3).unwrap().owner_pid, other_pid);
        assert_eq!(scheduler.task_state(1), None);
        assert_eq!(scheduler.task_state(2), None);
        assert_eq!(scheduler.task_state(3), Some(TaskState::Running));
        assert_eq!(scheduler.current_tid(), Some(3));
    }

    #[test]
    fn process_trap_runtime_reaps_exited_process() {
        let mut runtime = ProcessTrapRuntime::<3, 2, 2, 4>::new();
        let owner_pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let other_pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();

        for (tid, pid) in [(1, owner_pid), (2, owner_pid), (3, other_pid)] {
            let mut frame = TrapFrame::default();
            frame.sepc = 0x4000_0000 + tid * 0x1000;
            runtime.add_runnable_task(tid, pid, frame).unwrap();
        }
        runtime.start_first_task();

        let mut live = TrapFrame::default();
        let _ = runtime.dispatch_exit(&mut live, 1, 34);
        let report = runtime.reap_process(owner_pid).unwrap();

        assert_eq!(report.exit_code, 34);
        assert_eq!(report.tasks_reaped, 2);
        assert_eq!(report.scheduler_tasks_reaped, 2);
        assert_eq!(runtime.process_state(owner_pid), Ok(ProcState::Reaped));
        assert_eq!(runtime.task_state(1), None);
        assert_eq!(runtime.task_state(2), None);
        assert_eq!(runtime.task_state(3), Some(TaskState::Running));
        assert_eq!(runtime.current_tid(), Some(3));
    }

    #[test]
    fn process_wait_reaps_first_zombie_child() {
        let mut processes = ProcessTable::<4, 2, 2>::new();
        let mut tasks = TaskTable::<5>::new();
        let mut scheduler = Scheduler::<5>::new();
        let parent_pid = processes
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let child_pid = processes
            .create_process(Some(parent_pid), VmSpace::<2>::new())
            .unwrap();
        let other_pid = processes
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        processes
            .get_mut(parent_pid)
            .unwrap()
            .add_child(child_pid)
            .unwrap();

        for (tid, pid) in [(1, child_pid), (2, child_pid), (3, other_pid)] {
            let frame = TrapFrame::default();
            tasks
                .add_task(TaskControlBlock::new(tid, pid, frame))
                .unwrap();
            scheduler.add_runnable(tid, frame).unwrap();
        }
        let _ = scheduler.on_tick();
        let _ = exit_process_from_task_with_scheduler(
            &mut processes,
            &mut tasks,
            &mut scheduler,
            1,
            8,
        )
        .unwrap();

        let report =
            wait_reap_zombie_child(&mut processes, &mut tasks, &mut scheduler, parent_pid)
                .unwrap();

        assert_eq!(report.parent_pid, parent_pid);
        assert_eq!(report.child_pid, child_pid);
        assert_eq!(report.exit_code, 8);
        assert_eq!(report.tasks_reaped, 2);
        assert_eq!(report.scheduler_tasks_reaped, 2);
        assert_eq!(processes.get(child_pid).unwrap().state, ProcState::Reaped);
        assert_eq!(processes.get(parent_pid).unwrap().children[0], None);
        assert!(tasks.get(1).is_err());
        assert!(tasks.get(2).is_err());
        assert_eq!(scheduler.task_state(3), Some(TaskState::Running));
    }

    #[test]
    fn process_trap_runtime_wait_reaps_child() {
        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let parent_pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let child_pid = runtime
            .create_process(Some(parent_pid), VmSpace::<2>::new())
            .unwrap();
        let other_pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();

        for (tid, pid) in [(1, child_pid), (2, child_pid), (3, other_pid)] {
            let mut frame = TrapFrame::default();
            frame.sepc = 0x4000_0000 + tid * 0x1000;
            runtime.add_runnable_task(tid, pid, frame).unwrap();
        }
        runtime.start_first_task();

        let mut live = TrapFrame::default();
        let _ = runtime.dispatch_exit(&mut live, 1, 12);
        let report = runtime.wait_reap_child(parent_pid).unwrap();

        assert_eq!(report.parent_pid, parent_pid);
        assert_eq!(report.child_pid, child_pid);
        assert_eq!(report.exit_code, 12);
        assert_eq!(report.tasks_reaped, 2);
        assert_eq!(report.scheduler_tasks_reaped, 2);
        assert_eq!(runtime.process_state(child_pid), Ok(ProcState::Reaped));
        assert_eq!(runtime.task_state(1), None);
        assert_eq!(runtime.task_state(2), None);
        assert_eq!(runtime.task_state(3), Some(TaskState::Running));
        assert_eq!(runtime.current_tid(), Some(3));
    }

    #[test]
    fn process_trap_runtime_wait_uses_current_process_as_parent() {
        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let parent_pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let child_pid = runtime
            .create_process(Some(parent_pid), VmSpace::<2>::new())
            .unwrap();

        for (tid, pid) in [(2, child_pid), (3, child_pid), (1, parent_pid)] {
            let mut frame = TrapFrame::default();
            frame.sepc = 0x4000_0000 + tid * 0x1000;
            runtime.add_runnable_task(tid, pid, frame).unwrap();
        }
        runtime.start_first_task();

        let mut live = TrapFrame::default();
        let _ = runtime.dispatch_exit(&mut live, 2, 21);
        assert_eq!(runtime.current_process_id(), Ok(parent_pid));
        let report = runtime.wait_reap_current_child().unwrap();

        assert_eq!(report.parent_pid, parent_pid);
        assert_eq!(report.child_pid, child_pid);
        assert_eq!(report.exit_code, 21);
        assert_eq!(runtime.process_state(child_pid), Ok(ProcState::Reaped));
    }

    #[test]
    fn process_trap_runtime_wait_blocks_until_child_exit_wakes_parent() {
        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let parent_pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();
        let child_pid = runtime
            .create_process(Some(parent_pid), VmSpace::<2>::new())
            .unwrap();

        for (tid, pid) in [(1, parent_pid), (2, child_pid)] {
            let mut frame = TrapFrame::default();
            frame.sepc = 0x4000_0000 + tid * 0x1000;
            runtime.add_runnable_task(tid, pid, frame).unwrap();
        }
        runtime.start_first_task();

        let mut parent_frame = TrapFrame::default();
        let wait = runtime
            .wait_reap_current_child_or_block(&mut parent_frame)
            .unwrap();
        assert_eq!(
            wait,
            ProcessWaitSyscallRuntime::Blocked(TrapDispatchResult::Switched(
                ScheduleDecision::Switched {
                    prev_tid: 1,
                    next_tid: 2,
                }
            ))
        );
        assert_eq!(runtime.task_state(1), Some(TaskState::Blocked));
        assert_eq!(runtime.current_tid(), Some(2));

        let mut child_frame = TrapFrame::default();
        assert_eq!(
            runtime.dispatch_exit(&mut child_frame, 2, 31),
            TrapDispatchResult::Switched(ScheduleDecision::Switched {
                prev_tid: 2,
                next_tid: 1,
            })
        );
        assert_eq!(runtime.task_state(1), Some(TaskState::Running));
        assert_eq!(runtime.current_tid(), Some(1));

        let report = runtime.wait_reap_current_child().unwrap();
        assert_eq!(report.parent_pid, parent_pid);
        assert_eq!(report.child_pid, child_pid);
        assert_eq!(report.exit_code, 31);
    }

    #[test]
    fn process_trap_runtime_wait_without_children_returns_no_child() {
        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let parent_pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();

        let mut frame = TrapFrame::default();
        frame.sepc = 0x4000_0000;
        runtime.add_runnable_task(1, parent_pid, frame).unwrap();
        runtime.start_first_task();

        let mut wait_frame = TrapFrame::default();
        assert_eq!(
            runtime.wait_reap_current_child_or_block(&mut wait_frame),
            Err(ProcError::NoChild)
        );
        assert_eq!(process_wait_errno(ProcError::NoChild), ECHILD);
        assert_eq!(runtime.task_state(1), Some(TaskState::Running));
        assert_eq!(runtime.current_tid(), Some(1));
    }

    #[test]
    fn process_trap_runtime_fork_current_process_links_child_and_wait() {
        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let parent_pid = runtime
            .create_process(None, VmSpace::<2>::new())
            .unwrap();

        let mut parent_frame = TrapFrame::default();
        parent_frame.sepc = 0x4000_0000;
        parent_frame.set_sp(0x4010_0000);
        runtime.add_runnable_task(1, parent_pid, parent_frame).unwrap();
        runtime.start_first_task();

        let mut child_frame = TrapFrame::default();
        child_frame.sepc = 0x4000_1000;
        child_frame.set_sp(0x4010_1000);
        child_frame.set_a0(0xfeed);
        let fork = runtime
            .fork_current_process(2, child_frame, VmSpace::<2>::new())
            .unwrap();

        assert_eq!(fork.parent_pid, parent_pid);
        assert_eq!(fork.child_tid, 2);
        assert_eq!(
            handle_process_fork_syscall(fork),
            SyscallResult::Continue(fork.child_pid as isize)
        );
        assert_eq!(runtime.current_process_has_children(), Ok(true));
        assert_eq!(runtime.task_state(2), Some(TaskState::Runnable));
        assert_eq!(runtime.task_frame(2).unwrap().a0(), 0);

        let mut waiting_parent_frame = TrapFrame::default();
        assert_eq!(
            runtime
                .wait_reap_current_child_or_block(&mut waiting_parent_frame)
                .unwrap(),
            ProcessWaitSyscallRuntime::Blocked(TrapDispatchResult::Switched(
                ScheduleDecision::Switched {
                    prev_tid: 1,
                    next_tid: 2,
                }
            ))
        );

        let mut exiting_child_frame = TrapFrame::default();
        assert_eq!(
            runtime.dispatch_exit(&mut exiting_child_frame, 2, 23),
            TrapDispatchResult::Switched(ScheduleDecision::Switched {
                prev_tid: 2,
                next_tid: 1,
            })
        );

        let wait = runtime.wait_reap_current_child().unwrap();
        assert_eq!(wait.parent_pid, parent_pid);
        assert_eq!(wait.child_pid, fork.child_pid);
        assert_eq!(wait.exit_code, 23);
    }

    #[test]
    fn clone_syscall_surface_decodes_and_forks_current_process() {
        assert_eq!(
            decode_syscall(SYS_CLONE, 0, 0x4010_2000, 0),
            Some(Syscall::Clone {
                flags: 0,
                stack_ptr: 0x4010_2000,
            })
        );

        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let parent_pid = runtime.create_process(None, VmSpace::<2>::new()).unwrap();

        let mut parent_frame = TrapFrame::default();
        parent_frame.sepc = 0x4000_0000;
        parent_frame.set_sp(0x4010_0000);
        runtime.add_runnable_task(1, parent_pid, parent_frame).unwrap();
        runtime.start_first_task();

        let result = handle_current_process_clone_syscall(
            &mut runtime,
            decode_syscall(SYS_CLONE, 0, 0x4010_2000, 0).unwrap(),
            &parent_frame,
            2,
            VmSpace::<2>::new(),
        );

        assert_eq!(result, SyscallResult::Continue(2));
        assert_eq!(runtime.current_process_has_children(), Ok(true));
        assert_eq!(runtime.task_state(2), Some(TaskState::Runnable));
        let child_frame = runtime.task_frame(2).unwrap();
        assert_eq!(child_frame.a0(), 0);
        assert_eq!(child_frame.sp(), 0x4010_2000);
    }

    #[test]
    fn process_trap_runtime_exec_task_image_replaces_child_context() {
        let mut runtime = ProcessTrapRuntime::<4, 2, 2, 5>::new();
        let parent_pid = runtime.create_process(None, VmSpace::<2>::new()).unwrap();

        let mut parent_frame = TrapFrame::default();
        parent_frame.sepc = 0x4000_0000;
        parent_frame.set_sp(0x4010_0000);
        runtime.add_runnable_task(1, parent_pid, parent_frame).unwrap();
        runtime.start_first_task();

        let mut child_frame = TrapFrame::default();
        child_frame.sepc = 0x4000_1000;
        child_frame.set_sp(0x4010_1000);
        let fork = runtime
            .fork_current_process(2, child_frame, VmSpace::<2>::new())
            .unwrap();

        let image = UserExecImage {
            entry: 0x4000_2000,
            stack_top: 0x4010_2000,
            segment_count: 1,
            segments: [
                Some(SegmentPlacement {
                    vaddr: 0x4000_2000,
                    file_size: 16,
                    mem_size: 32,
                    flags: (PTE_R | PTE_X) as u32,
                }),
                None,
                None,
                None,
            ],
        };
        let exec = runtime
            .exec_task_image(fork.child_tid, image, VmSpace::<2>::new())
            .unwrap();

        assert_eq!(exec.pid, fork.child_pid);
        assert_eq!(exec.tid, fork.child_tid);
        assert_eq!(exec.entry, 0x4000_2000);
        assert_eq!(exec.stack_top, 0x4010_2000);
        assert_eq!(handle_process_execv_syscall(exec), SyscallResult::Continue(0));

        let frame = runtime.task_frame(fork.child_tid).unwrap();
        assert_eq!(frame.sepc, exec.entry);
        assert_eq!(frame.sp(), exec.stack_top);
        assert_eq!(frame.sstatus & super::user_init::SSTATUS_SPP, 0);
        assert_ne!(frame.sstatus & super::user_init::SSTATUS_SPIE, 0);
    }

    #[test]
    fn sv39_leaf_entry_keeps_ppn_and_flags() {
        let pte = PageTableEntry::leaf(0x80200, PTE_R | PTE_W | PTE_X | PTE_A | PTE_D);
        assert_eq!(pte.ppn(), 0x80200);
        assert_eq!(pte.flags() & (PTE_R | PTE_W | PTE_X), PTE_R | PTE_W | PTE_X);
    }

    #[test]
    fn sv39_satp_has_mode_8() {
        let satp = make_satp(0x12345, 0);
        assert_eq!(satp >> 60, 8);
    }

    #[test]
    fn sv39_root_table_maps_identity_entry() {
        let mut root = RootPageTable::new();
        root.map_gib_identity(2, 0x20000, PTE_R | PTE_X);
        assert_eq!(root.entries[2].ppn(), 0x20000);
    }

    #[test]
    fn sv39_root_table_storage_is_page_aligned_and_yields_ppn() {
        let root = RootPageTable::new();
        assert_eq!(core::mem::size_of::<RootPageTable>(), super::sv39::PAGE_SIZE);
        assert!(root.is_page_aligned());
        assert_eq!(root.ppn(), root.physical_addr() / super::sv39::PAGE_SIZE);
    }


    #[test]
    fn kernel_identity_mapping_is_global_and_supervisor_only() {
        let mut root = RootPageTable::new();
        install_kernel_identity_mapping(&mut root).unwrap();
        let entry = root.entries[super::user_vm::KERNEL_IDENTITY_VPN2];
        let mmio = root.entries[super::user_vm::KERNEL_MMIO_VPN2];

        assert!(is_kernel_mapping(entry));
        assert_eq!(entry.flags() & PTE_U, 0);
        assert_ne!(entry.flags() & PTE_G, 0);
        assert_ne!(entry.flags() & PTE_R, 0);
        assert_ne!(entry.flags() & PTE_W, 0);
        assert_ne!(entry.flags() & PTE_X, 0);
        assert!(mmio.is_valid());
        assert_eq!(mmio.ppn(), super::user_vm::KERNEL_MMIO_PPN);
        assert_eq!(mmio.flags() & PTE_U, 0);
        assert_ne!(mmio.flags() & PTE_G, 0);
        assert_ne!(mmio.flags() & PTE_R, 0);
        assert_ne!(mmio.flags() & PTE_W, 0);
        assert_eq!(mmio.flags() & PTE_X, 0);
    }

    #[test]
    fn relocated_user_vm_plan_can_preserve_kernel_mapping() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut allocator = PageFrameAllocator::new(0x9000_0000, 0x9000_1000, super::sv39::PAGE_SIZE);
        let root_frame_pa = allocator.alloc().unwrap();
        let mut root = RootPageTable::new();
        install_user_vm_plan_in_root(&loaded, root_frame_pa, &mut root).unwrap();

        let user_entry = root.entries[1];
        assert!(is_leaf_user_mapping(user_entry));
        assert!(install_kernel_identity_mapping(&mut root).is_ok());
        assert!(is_kernel_mapping(root.entries[super::user_vm::KERNEL_IDENTITY_VPN2]));
    }

    #[test]
    fn user_vm_plan_can_be_installed_in_aligned_root_storage() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut allocator = PageFrameAllocator::new(0x9000_0000, 0x9000_1000, super::sv39::PAGE_SIZE);
        let root_frame_pa = allocator.alloc().unwrap();
        let mut root = RootPageTable::new();
        let install = install_user_vm_plan_in_root(&loaded, root_frame_pa, &mut root).unwrap();

        assert_eq!(root_frame_pa % super::sv39::PAGE_SIZE, 0);
        assert_eq!(install.root_frame_pa, root_frame_pa);
        assert_eq!(install.root_ppn, root_frame_pa / super::sv39::PAGE_SIZE);
        assert_eq!(install.satp, make_satp(root_frame_pa / super::sv39::PAGE_SIZE, 0));
        assert!(is_leaf_user_mapping(root.entries[1]));
    }

    #[test]
    fn process_address_space_records_root_frame_and_satp() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut first_space = ProcessAddressSpace::empty();
        let mut second_space = ProcessAddressSpace::empty();
        let mut roots = [RootPageTable::new(), RootPageTable::new()];
        let mut allocator =
            PageFrameAllocator::new(0x9000_0000, 0x9000_2000, super::sv39::PAGE_SIZE);
        let first_frame = allocator.alloc().unwrap();
        let second_frame = allocator.alloc().unwrap();

        let first_install = first_space
            .install_user_image(&loaded, first_frame, &mut roots[0])
            .unwrap();
        let second_install = second_space
            .install_user_image(&loaded, second_frame, &mut roots[1])
            .unwrap();

        assert_eq!(first_space.root_frame_pa, first_frame);
        assert_eq!(second_space.root_frame_pa, second_frame);
        assert_eq!(first_space.satp, first_install.satp);
        assert_eq!(second_space.satp, second_install.satp);
        assert_eq!(first_space.mapped_regions, first_install.mapped_regions);
        assert_eq!(second_space.mapped_regions, second_install.mapped_regions);
        assert_eq!(
            first_space.satp,
            make_satp(first_space.root_frame_pa / super::sv39::PAGE_SIZE, 0)
        );
        assert_eq!(
            second_space.satp,
            make_satp(second_space.root_frame_pa / super::sv39::PAGE_SIZE, 0)
        );
        assert_ne!(first_space.root_frame_pa, second_space.root_frame_pa);
        assert_ne!(first_space.satp, second_space.satp);
        assert!(is_leaf_user_mapping(roots[0].entries[1]));
        assert!(is_leaf_user_mapping(roots[1].entries[1]));
    }

    #[test]
    fn pcb_address_space_handle_resolves_switch_satp_from_space_table() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut first_stack = [0u8; 512];
        let mut second_stack = [0u8; 512];
        let first_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
            0x12345_000,
            &mut first_stack,
            &["/sbin/init"],
            &["PATH=/bin"],
        )
        .unwrap();
        let second_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
            0x12346_000,
            &mut second_stack,
            &["/sbin/worker"],
            &["PATH=/bin"],
        )
        .unwrap();

        let mut spaces = ProcessAddressSpaceTable::<4>::new();
        let second_space_id = spaces.create_empty().unwrap();
        let mut second_root = RootPageTable::new();
        let mut allocator =
            PageFrameAllocator::new(0x9000_0000, 0x9000_1000, super::sv39::PAGE_SIZE);
        let second_root_frame = allocator.alloc().unwrap();
        let second_install = spaces
            .install_user_image(second_space_id, &loaded, second_root_frame, &mut second_root)
            .unwrap();
        let mut proc_table = ProcessTable::<4, 4, 4>::new();
        let mut task_table = TaskTable::<4>::new();
        let mut scheduler = Scheduler::<4>::new();
        let first = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            None,
            VmSpace::<4>::new(),
            first_launch,
        )
        .unwrap();
        let second = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            Some(first.pid),
            VmSpace::<4>::new(),
            second_launch,
        )
        .unwrap();

        proc_table
            .bind_address_space(second.pid, second_space_id)
            .unwrap();
        scheduler.on_tick();
        scheduler.set_state(first.tid, TaskState::Blocked);
        let decision =
            schedule_after_blocked_syscall(&first_launch.trapframe, &mut scheduler).unwrap();
        let switch = address_space_switch_for_decision_with_spaces(
            &proc_table,
            &task_table,
            &spaces,
            decision,
        )
        .unwrap()
        .unwrap();

        let second_pcb = proc_table.get(second.pid).unwrap();
        assert_eq!(second_pcb.address_space_id, Some(second_space_id));
        assert_eq!(second_pcb.satp, second_launch.satp);
        assert_eq!(spaces.satp(second_space_id).unwrap(), second_install.satp);
        assert_eq!(switch.next_tid, second.tid);
        assert_eq!(switch.next_satp, second_install.satp);
        assert_ne!(switch.next_satp, second_pcb.satp);
    }

    #[test]
    fn satp_commit_preflight_requires_user_and_kernel_mappings() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut allocator = PageFrameAllocator::new(0x9000_0000, 0x9000_1000, super::sv39::PAGE_SIZE);
        let root_frame_pa = allocator.alloc().unwrap();
        let mut root = RootPageTable::new();
        install_user_vm_plan_in_root(&loaded, root_frame_pa, &mut root).unwrap();
        install_kernel_identity_mapping(&mut root).unwrap();

        assert_eq!(
            preflight_satp_commit(&root, loaded.entry, loaded.stack_top),
            Ok(SatpCommitPreflight {
                user_entry_vpn2: 1,
                user_stack_vpn2: 1,
                kernel_vpn2: super::user_vm::KERNEL_IDENTITY_VPN2,
            })
        );
    }

    #[test]
    fn satp_commit_preflight_rejects_missing_kernel_mapping() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut allocator = PageFrameAllocator::new(0x9000_0000, 0x9000_1000, super::sv39::PAGE_SIZE);
        let root_frame_pa = allocator.alloc().unwrap();
        let mut root = RootPageTable::new();
        install_user_vm_plan_in_root(&loaded, root_frame_pa, &mut root).unwrap();

        assert_eq!(
            preflight_satp_commit(&root, loaded.entry, loaded.stack_top),
            Err(SatpCommitPreflightError::MissingKernelMapping {
                vpn2: super::user_vm::KERNEL_IDENTITY_VPN2,
            })
        );
    }

    #[test]
    fn kernel_runtime_satp_guard_requires_code_stack_and_trap_entry_mapping() {
        let mut root = RootPageTable::new();
        install_kernel_identity_mapping(&mut root).unwrap();

        assert_eq!(
            guard_kernel_runtime_after_satp(
                &root,
                0x8020_0000,
                0x8020_8000 - 16,
                0x8020_1000,
            ),
            Ok(KernelRuntimeSatpGuard {
                kernel_code_vpn2: super::user_vm::KERNEL_IDENTITY_VPN2,
                kernel_stack_vpn2: super::user_vm::KERNEL_IDENTITY_VPN2,
                trap_entry_vpn2: super::user_vm::KERNEL_IDENTITY_VPN2,
            })
        );
    }

    #[test]
    fn kernel_runtime_satp_guard_rejects_missing_kernel_mapping() {
        let root = RootPageTable::new();

        assert_eq!(
            guard_kernel_runtime_after_satp(
                &root,
                0x8020_0000,
                0x8020_8000 - 16,
                0x8020_1000,
            ),
            Err(KernelRuntimeSatpGuardError::MissingKernelCode {
                vpn2: super::user_vm::KERNEL_IDENTITY_VPN2,
            })
        );
    }

    #[test]
    fn user_init_context_clears_spp_for_umode() {
        let ctx = build_user_init_context(0x4000_0000, 0x4010_0000);
        assert_eq!(ctx.sepc, 0x4000_0000);
        assert_eq!(ctx.sstatus & SSTATUS_SPP, 0);
    }

    #[test]
    fn user_init_demo_exits_with_zero() {
        assert_eq!(run_user_init_demo(), TrapOutcome::Exit(0));
    }

    #[test]
    fn user_exec_materialize_copies_file_and_zeros_bss() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let seg = loaded.segments[0].unwrap();
        let mut region = vec![0xaa_u8; seg.mem_size];
        let mut regions = [UserRegionMut {
            vaddr: seg.vaddr,
            bytes: region.as_mut_slice(),
        }];

        materialize_segments(&loaded, &mut regions).unwrap();

        assert_eq!(&region[..4], &[0x97, 0x05, 0x00, 0x00]);
        assert!(region
            .windows(b"[user-init]".len())
            .any(|w| w == b"[user-init]"));
    }

    #[test]
    fn user_exec_materialize_fails_when_region_is_missing() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut region = [0u8; 8];
        let mut regions = [UserRegionMut {
            vaddr: 0x9000_0000,
            bytes: &mut region,
        }];

        let seg = loaded.segments[0].unwrap();
        assert_eq!(
            materialize_segments(&loaded, &mut regions),
            Err(ExecError::MissingRegion(seg.vaddr))
        );
    }

    #[test]
    fn user_exec_context_sets_entry_stack_and_umode_bits() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let image = UserExecImage::from_loaded(&loaded);
        let mut tf = TrapFrame::default();
        tf.sstatus = usize::MAX;

        install_exec_context(&mut tf, &image);

        assert_eq!(tf.sepc, 0x4000_0000);
        assert_eq!(tf.sp(), 0x4010_0000);
        assert_eq!(tf.sstatus & SSTATUS_SPP, 0);
        assert_ne!(tf.sstatus & super::user_init::SSTATUS_SPIE, 0);
    }

    #[test]
    fn user_exec_can_load_init_from_vfs_and_prepare_image() {
        let image = build_test_ext4_image();
        let dev = RamDisk::new("ramblk0", &image, 512);
        let vfs = Vfs::mount_ext4(&dev).unwrap();
        let mut scratch = [0u8; 512];

        let loaded = load_init_exec_image(&vfs, "/sbin/init", &mut scratch, 0x4010_0000).unwrap();
        let exec = UserExecImage::from_loaded(&loaded);

        assert_eq!(exec.entry, 0x4000_0000);
        assert_eq!(exec.segment_count, 1);
        let seg = exec.segments[0].unwrap();
        assert_eq!(seg.vaddr, 0x4000_0000);
        assert!(seg.file_size >= 64);
        assert_eq!(seg.mem_size, seg.file_size);
    }

    #[test]
    fn user_stack_builder_writes_argc_argv_envp_layout() {
        let stack_top = 0x4010_0000usize;
        let mut stack = [0u8; 512];
        let layout = build_initial_user_stack::<8, 8>(
            &mut stack,
            stack_top,
            &["/sbin/init", "--verbose"],
            &["PATH=/bin", "TERM=vt100"],
        )
        .unwrap();

        let base = stack_top - stack.len();
        let sp_off = layout.sp - base;
        let argc = read_usize_at(&stack, sp_off);
        assert_eq!(argc, 2);

        let argv0 = read_usize_at(&stack, sp_off + core::mem::size_of::<usize>());
        let argv1 = read_usize_at(&stack, sp_off + 2 * core::mem::size_of::<usize>());
        let argv_null = read_usize_at(&stack, sp_off + 3 * core::mem::size_of::<usize>());
        assert_eq!(argv_null, 0);
        assert_eq!(read_cstr_at(&stack, base, argv0), b"/sbin/init");
        assert_eq!(read_cstr_at(&stack, base, argv1), b"--verbose");

        let envp0 = read_usize_at(&stack, sp_off + 4 * core::mem::size_of::<usize>());
        let envp1 = read_usize_at(&stack, sp_off + 5 * core::mem::size_of::<usize>());
        let envp_null = read_usize_at(&stack, sp_off + 6 * core::mem::size_of::<usize>());
        assert_eq!(envp_null, 0);
        assert_eq!(read_cstr_at(&stack, base, envp0), b"PATH=/bin");
        assert_eq!(read_cstr_at(&stack, base, envp1), b"TERM=vt100");
    }

    #[test]
    fn user_stack_builder_rejects_small_buffer() {
        let mut stack = [0u8; 32];
        let err = build_initial_user_stack::<2, 2>(
            &mut stack,
            0x4010_0000,
            &["/sbin/init", "--verbose"],
            &["PATH=/bin", "TERM=vt100"],
        )
        .unwrap_err();
        assert_eq!(err, UserStackError::StackOverflow);
    }

    #[test]
    fn user_vm_plan_maps_exec_and_stack_regions() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut allocator = PageFrameAllocator::new(0x9000_0000, 0x9000_1000, super::sv39::PAGE_SIZE);
        let root_frame_pa = allocator.alloc().unwrap();
        let mut root = RootPageTable::new();
        let install = install_user_vm_plan_in_root(&loaded, root_frame_pa, &mut root).unwrap();
        let plan = build_user_vm_plan(&loaded, root_frame_pa).unwrap();

        assert_eq!(plan.satp >> 60, 8);
        assert_eq!(plan.mapped_regions, 1);
        assert_eq!(install.satp, plan.satp);
        assert_eq!(install.root_frame_pa, root_frame_pa);

        // 0x4000_0000 and stack 0x4010_0000 both live under vpn2=1.
        let entry = root.entries[1];
        assert!(is_leaf_user_mapping(entry));
        assert_ne!(entry.flags() & PTE_X, 0);
        assert_ne!(entry.flags() & PTE_W, 0);
        assert_ne!(entry.flags() & PTE_R, 0);
        assert_ne!(entry.flags() & PTE_V, 0);
    }

    #[test]
    fn user_vm_plan_can_map_multiple_gib_regions() {
        let data0 = [0xaa_u8; 4];
        let data1 = [0xbb_u8; 4];
        let loaded = super::elf_loader::LoadedUserImage {
            entry: 0x4000_0000,
            stack_top: 0x8000_1000,
            segments: [
                Some(super::elf_loader::LoadSegment {
                    vaddr: 0x4000_0000,
                    file_size: 4,
                    mem_size: 4,
                    flags: 5,
                    data: &data0,
                }),
                Some(super::elf_loader::LoadSegment {
                    vaddr: 0x8000_0000,
                    file_size: 4,
                    mem_size: 4,
                    flags: 6,
                    data: &data1,
                }),
                None,
                None,
            ],
            segment_count: 2,
        };

        let mut allocator = PageFrameAllocator::new(0x9000_0000, 0x9000_1000, super::sv39::PAGE_SIZE);
        let root_frame_pa = allocator.alloc().unwrap();
        let mut root = RootPageTable::new();
        let plan = build_user_vm_plan(&loaded, root_frame_pa).unwrap();
        install_user_vm_plan_in_root(&loaded, root_frame_pa, &mut root).unwrap();
        assert_eq!(plan.mapped_regions, 2);
        assert!(is_leaf_user_mapping(root.entries[1]));
        assert!(is_leaf_user_mapping(root.entries[2]));
    }

    #[test]
    fn user_launch_plan_combines_vm_exec_and_stack_abi() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut stack = [0u8; 512];
        let launch = prepare_user_init_launch::<8, 8>(
            &loaded,
            0x12345_000,
            &mut stack,
            &["/sbin/init", "--verbose"],
            &["PATH=/bin"],
        )
        .unwrap();

        assert_eq!(launch.satp >> 60, 8);
        assert_eq!(launch.mapped_regions, 1);
        assert_eq!(launch.trapframe.sepc, loaded.entry);
        assert_eq!(launch.trapframe.sp(), launch.sp);
        assert_eq!(launch.trapframe.a0(), 2);
        assert_eq!(launch.trapframe.a1(), launch.argv_ptr);
        assert_eq!(launch.trapframe.a2(), launch.envp_ptr);
    }


    #[test]
    fn user_launch_task_is_installed_into_global_trap_runtime() {
        reset_global_trap_runtime();

        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut stack = [0u8; 512];
        let launch = prepare_user_init_launch::<8, 8>(
            &loaded,
 0x12345_000,
            &mut stack,
            &["/sbin/init"],
            &["PATH=/bin"],
        )
        .unwrap();

        install_initial_user_task(1, launch.trapframe).unwrap();

        assert_eq!(
            with_global_trap_runtime(|runtime| runtime.current_tid()),
            Some(1)
        );
        let installed = with_global_trap_runtime(|runtime| runtime.task_frame(1).unwrap());
        assert_eq!(installed.sepc, loaded.entry);
        assert_eq!(installed.sp(), launch.sp);
        assert_eq!(installed.a0(), 1);
        assert_eq!(installed.a1(), launch.argv_ptr);
        assert_eq!(installed.a2(), launch.envp_ptr);

        reset_global_trap_runtime();
    }


    #[test]
    fn user_process_creation_registers_launch_context() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut stack = [0u8; 512];
        let launch = prepare_user_init_launch::<8, 8>(
            &loaded,
 0x12345_000,
            &mut stack,
            &["/sbin/init"],
            &["PATH=/bin"],
        )
        .unwrap();

        let mut proc_table = ProcessTable::<4, 4, 4>::new();
        let mut task_table = TaskTable::<4>::new();
        let mut scheduler = Scheduler::<4>::new();
        let created = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            None,
            VmSpace::<4>::new(),
            launch,
        )
        .unwrap();

        assert_eq!(created.pid, 1);
        assert_eq!(created.tid, 1);
        assert_eq!(created.entry, loaded.entry);
        assert_eq!(created.stack_top, launch.sp);
        assert_eq!(created.satp, launch.satp);
        assert_eq!(proc_table.get_mut(created.pid).unwrap().pid, created.pid);
        let task = task_table.get_mut(created.tid).unwrap();
        assert_eq!(task.owner_pid, created.pid);
        assert_eq!(task.trapframe.sepc, loaded.entry);
        assert_eq!(task.trapframe.sp(), launch.sp);
        assert_eq!(proc_table.get(created.pid).unwrap().satp, launch.satp);
        assert_eq!(scheduler.task_state(created.tid), Some(TaskState::Runnable));
        assert_eq!(scheduler.task_frame(created.tid).unwrap().sepc, loaded.entry);
    }

    #[test]
    fn two_created_user_processes_can_switch_after_block() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut first_stack = [0u8; 512];
        let mut second_stack = [0u8; 512];
        let first_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
 0x12345_000,
            &mut first_stack,
            &["/sbin/init"],
            &["PATH=/bin"],
        )
        .unwrap();
        let second_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
 0x12346_000,
            &mut second_stack,
            &["/sbin/worker"],
            &["PATH=/bin"],
        )
        .unwrap();

        let mut proc_table = ProcessTable::<4, 4, 4>::new();
        let mut task_table = TaskTable::<4>::new();
        let mut scheduler = Scheduler::<4>::new();
        let first = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            None,
            VmSpace::<4>::new(),
            first_launch,
        )
        .unwrap();
        let second = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            Some(first.pid),
            VmSpace::<4>::new(),
            second_launch,
        )
        .unwrap();

        scheduler.on_tick();
        assert_eq!(scheduler.current_tid(), Some(first.tid));
        scheduler.set_state(first.tid, TaskState::Blocked);

        let mut live = first_launch.trapframe;
        live.sepc = 0xfeed_0000;
        let decision = schedule_after_blocked_syscall(&live, &mut scheduler).unwrap();
        assert_eq!(
            decision,
            ScheduleDecision::Switched {
                prev_tid: first.tid,
                next_tid: second.tid,
            }
        );
        assert_eq!(
            apply_schedule_decision(&mut live, &scheduler, decision),
            Some(decision)
        );

        assert_eq!(scheduler.current_tid(), Some(second.tid));
        assert_eq!(scheduler.task_state(first.tid), Some(TaskState::Blocked));
        assert_eq!(scheduler.task_state(second.tid), Some(TaskState::Running));
        assert_eq!(scheduler.task_frame(first.tid).unwrap().sepc, 0xfeed_0000);
        assert_eq!(live.sepc, second_launch.trapframe.sepc);
        assert_eq!(live.sp(), second_launch.sp);
        assert_eq!(task_table.get_mut(second.tid).unwrap().owner_pid, second.pid);
        assert_eq!(proc_table.get_mut(first.pid).unwrap().children[0], Some(second.pid));
    }

    #[test]
    fn switched_task_exposes_next_address_space_token() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut first_stack = [0u8; 512];
        let mut second_stack = [0u8; 512];
        let first_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
 0x12345_000,
            &mut first_stack,
            &["/sbin/init"],
            &["PATH=/bin"],
        )
        .unwrap();
        let second_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
 0x12346_000,
            &mut second_stack,
            &["/sbin/worker"],
            &["PATH=/bin"],
        )
        .unwrap();

        let mut proc_table = ProcessTable::<4, 4, 4>::new();
        let mut task_table = TaskTable::<4>::new();
        let mut scheduler = Scheduler::<4>::new();
        let first = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            None,
            VmSpace::<4>::new(),
            first_launch,
        )
        .unwrap();
        let second = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            Some(first.pid),
            VmSpace::<4>::new(),
            second_launch,
        )
        .unwrap();

        scheduler.on_tick();
        scheduler.set_state(first.tid, TaskState::Blocked);
        let decision =
            schedule_after_blocked_syscall(&first_launch.trapframe, &mut scheduler).unwrap();
        let switch = address_space_switch_for_decision(&proc_table, &task_table, decision)
            .unwrap()
            .unwrap();

        assert_eq!(switch.prev_tid, first.tid);
        assert_eq!(switch.next_tid, second.tid);
        assert_eq!(switch.next_satp, second_launch.satp);
        assert_ne!(first_launch.satp, second_launch.satp);
    }

    #[test]
    fn process_address_space_token_can_be_rebound_to_root_storage_satp() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut first_stack = [0u8; 512];
        let mut second_stack = [0u8; 512];
        let first_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
            0x12345_000,
            &mut first_stack,
            &["/sbin/init"],
            &["PATH=/bin"],
        )
        .unwrap();
        let second_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
            0x12346_000,
            &mut second_stack,
            &["/sbin/worker"],
            &["PATH=/bin"],
        )
        .unwrap();
        let mut allocator = PageFrameAllocator::new(0x9000_0000, 0x9000_2000, super::sv39::PAGE_SIZE);
        let root_frame_pa = allocator.alloc().unwrap();
        let mut root = RootPageTable::new();
        let root_install = install_user_vm_plan_in_root(&loaded, root_frame_pa, &mut root).unwrap();

        let mut proc_table = ProcessTable::<4, 4, 4>::new();
        let mut task_table = TaskTable::<4>::new();
        let mut scheduler = Scheduler::<4>::new();
        let first = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            None,
            VmSpace::<4>::new(),
            first_launch,
        )
        .unwrap();
        let second = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            Some(first.pid),
            VmSpace::<4>::new(),
            second_launch,
        )
        .unwrap();

        proc_table.update_satp(second.pid, root_install.satp).unwrap();
        scheduler.on_tick();
        scheduler.set_state(first.tid, TaskState::Blocked);
        let decision =
            schedule_after_blocked_syscall(&first_launch.trapframe, &mut scheduler).unwrap();
        let switch = address_space_switch_for_decision(&proc_table, &task_table, decision)
            .unwrap()
            .unwrap();

        assert_eq!(proc_table.get(second.pid).unwrap().satp, root_install.satp);
        assert_eq!(switch.next_tid, second.tid);
        assert_eq!(switch.next_satp, root_install.satp);
        assert_ne!(switch.next_satp, second_launch.satp);
    }

    #[test]
    fn rooted_process_token_flows_into_commit_and_launch_sequence() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut first_stack = [0u8; 512];
        let mut second_stack = [0u8; 512];
        let first_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
            0x12345_000,
            &mut first_stack,
            &["/sbin/init"],
            &["PATH=/bin"],
        )
        .unwrap();
        let second_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
            0x12346_000,
            &mut second_stack,
            &["/sbin/worker"],
            &["PATH=/bin"],
        )
        .unwrap();
        let mut allocator = PageFrameAllocator::new(0x9000_0000, 0x9000_2000, super::sv39::PAGE_SIZE);
        let root_frame_pa = allocator.alloc().unwrap();
        let mut root = RootPageTable::new();
        let root_install = install_user_vm_plan_in_root(&loaded, root_frame_pa, &mut root).unwrap();

        let mut proc_table = ProcessTable::<4, 4, 4>::new();
        let mut task_table = TaskTable::<4>::new();
        let mut scheduler = Scheduler::<4>::new();
        let first = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            None,
            VmSpace::<4>::new(),
            first_launch,
        )
        .unwrap();
        let second = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            Some(first.pid),
            VmSpace::<4>::new(),
            second_launch,
        )
        .unwrap();

        proc_table.update_satp(second.pid, root_install.satp).unwrap();
        scheduler.on_tick();
        scheduler.set_state(first.tid, TaskState::Blocked);
        let decision =
            schedule_after_blocked_syscall(&first_launch.trapframe, &mut scheduler).unwrap();
        let switch = address_space_switch_for_decision(&proc_table, &task_table, decision)
            .unwrap()
            .unwrap();
        let mut frame = second_launch.trapframe;
        frame.set_sp(loaded.stack_top);
        let scratch = prepare_enter_user_scratch(&frame).unwrap();
        let mut active_satp = first_launch.satp;

        let sequence = prepare_rooted_satp_commit_sequence(
            &mut active_satp,
            switch,
            &frame,
            scratch,
            root_install.satp,
        )
        .unwrap();

        assert_eq!(active_satp, root_install.satp);
        assert_eq!(sequence.switch.next_satp, root_install.satp);
        assert_eq!(sequence.applied.active_satp, root_install.satp);
        assert_eq!(sequence.commit.next_satp, root_install.satp);
        assert!(sequence.commit.needs_sfence_vma);
        assert_eq!(sequence.rooted.root_satp, root_install.satp);
        assert_eq!(sequence.rooted.sequence.satp, root_install.satp);
        assert_eq!(sequence.rooted.sequence.sepc, frame.sepc);
    }


    #[test]
    fn address_space_switch_updates_active_satp_slot() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut first_stack = [0u8; 512];
        let first_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
 0x12345_000,
            &mut first_stack,
            &["/sbin/init"],
            &["PATH=/bin"],
        )
        .unwrap();
        let mut second_stack = [0u8; 512];
        let second_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
 0x12346_000,
            &mut second_stack,
            &["/sbin/worker"],
            &["PATH=/bin"],
        )
        .unwrap();

        let mut proc_table = ProcessTable::<4, 4, 4>::new();
        let mut task_table = TaskTable::<4>::new();
        let mut scheduler = Scheduler::<4>::new();
        let first = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            None,
            VmSpace::<4>::new(),
            first_launch,
        )
        .unwrap();
        let second = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            Some(first.pid),
            VmSpace::<4>::new(),
            second_launch,
        )
        .unwrap();

        scheduler.on_tick();
        scheduler.set_state(first.tid, TaskState::Blocked);
        let decision =
            schedule_after_blocked_syscall(&first_launch.trapframe, &mut scheduler).unwrap();
        let switch = address_space_switch_for_decision(&proc_table, &task_table, decision)
            .unwrap()
            .unwrap();
        let mut active_satp = first_launch.satp;
        let applied = apply_address_space_switch(&mut active_satp, switch);

        assert_eq!(applied.prev_tid, first.tid);
        assert_eq!(applied.next_tid, second.tid);
        assert_eq!(applied.active_satp, second_launch.satp);
        assert_eq!(active_satp, second_launch.satp);
    }


    #[test]
    fn satp_commit_plan_requires_tlb_flush_when_satp_changes() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut first_stack = [0u8; 512];
        let first_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
 0x12345_000,
            &mut first_stack,
            &["/sbin/init"],
            &["PATH=/bin"],
        )
        .unwrap();
        let mut second_stack = [0u8; 512];
        let second_launch = prepare_user_init_launch::<8, 8>(
            &loaded,
 0x12346_000,
            &mut second_stack,
            &["/sbin/worker"],
            &["PATH=/bin"],
        )
        .unwrap();

        let mut proc_table = ProcessTable::<4, 4, 4>::new();
        let mut task_table = TaskTable::<4>::new();
        let mut scheduler = Scheduler::<4>::new();
        let first = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            None,
            VmSpace::<4>::new(),
            first_launch,
        )
        .unwrap();
        let _second = create_user_process_from_launch(
            &mut proc_table,
            &mut task_table,
            &mut scheduler,
            Some(first.pid),
            VmSpace::<4>::new(),
            second_launch,
        )
        .unwrap();

        scheduler.on_tick();
        scheduler.set_state(first.tid, TaskState::Blocked);
        let decision =
            schedule_after_blocked_syscall(&first_launch.trapframe, &mut scheduler).unwrap();
        let switch = address_space_switch_for_decision(&proc_table, &task_table, decision)
            .unwrap()
            .unwrap();
        let mut active_satp = first_launch.satp;
        let applied = apply_address_space_switch(&mut active_satp, switch);
        let commit = prepare_satp_commit(first_launch.satp, applied);

        assert_eq!(commit.prev_satp, first_launch.satp);
        assert_eq!(commit.next_satp, second_launch.satp);
        assert!(commit.needs_sfence_vma);
    }

    #[test]
    fn satp_hardware_commit_plan_requires_sv39_and_changed_satp() {
        let changed = prepare_satp_commit(
            make_satp(0x11111, 0),
            super::task_vm::AppliedAddressSpaceSwitch {
                prev_tid: 1,
                next_tid: 2,
                active_satp: make_satp(0x22222, 0),
            },
        );
        let hardware = prepare_satp_hardware_commit(changed).unwrap();

        assert_eq!(hardware.next_satp, make_satp(0x22222, 0));
        assert!(hardware.flush_after_write);

        let unchanged = prepare_satp_commit(
            make_satp(0x33333, 0),
            super::task_vm::AppliedAddressSpaceSwitch {
                prev_tid: 1,
                next_tid: 1,
                active_satp: make_satp(0x33333, 0),
            },
        );
        assert_eq!(
            prepare_satp_hardware_commit(unchanged),
            Err(SatpHardwareCommitError::Unchanged)
        );

        let bare = prepare_satp_commit(
            make_satp(0x11111, 0),
            super::task_vm::AppliedAddressSpaceSwitch {
                prev_tid: 1,
                next_tid: 2,
                active_satp: 0x22222,
            },
        );
        assert_eq!(
            prepare_satp_hardware_commit(bare),
            Err(SatpHardwareCommitError::UnsupportedMode { mode: 0 })
        );
    }

    #[test]
    fn satp_hardware_commit_probe_roundtrips_to_restore_satp() {
        let changed = prepare_satp_commit(
            0,
            super::task_vm::AppliedAddressSpaceSwitch {
                prev_tid: 1,
                next_tid: 2,
                active_satp: make_satp(0x22222, 0),
            },
        );
        let hardware = prepare_satp_hardware_commit(changed).unwrap();
        let probe = unsafe { probe_satp_hardware_commit_roundtrip(hardware, 0) };

        assert_eq!(probe.committed_satp, make_satp(0x22222, 0));
        assert_eq!(probe.restored_satp, 0);
        assert!(probe.flushed_on_commit);
        assert!(probe.flushed_on_restore);
    }

    #[test]
    fn satp_hardware_commit_keeps_new_active_satp() {
        let changed = prepare_satp_commit(
            0,
            super::task_vm::AppliedAddressSpaceSwitch {
                prev_tid: 1,
                next_tid: 2,
                active_satp: make_satp(0x22222, 0),
            },
        );
        let hardware = prepare_satp_hardware_commit(changed).unwrap();
        let committed = unsafe { commit_satp_hardware(hardware) };

        assert_eq!(committed.active_satp, make_satp(0x22222, 0));
        assert!(committed.flushed_tlb);
    }

    #[test]
    fn first_user_sret_frame_can_be_stored_for_late_entry() {
        let mut frame = TrapFrame::default();
        frame.sepc = 0x4001_0000;
        frame.set_sp(0x4010_0000);
        frame.set_a0(1);
        frame.set_a1(0x4001_1000);
        frame.set_a2(4);
        frame.set_a7(SYS_WRITE);

        let stored = frame;

        assert_eq!(stored.sepc, 0x4001_0000);
        assert_eq!(stored.sp(), 0x4010_0000);
        assert_eq!(stored.a7(), SYS_WRITE);
    }

    #[test]
    fn user_launch_csr_sequence_orders_scratch_satp_flush_and_sret_state() {
        let mut frame = TrapFrame::default();
        frame.sepc = 0x4001_0000;
        frame.set_sp(0x4010_0000);
        let scratch = prepare_enter_user_scratch(&frame).unwrap();
        let commit = prepare_satp_commit(
            make_satp(0x11111, 0),
            super::task_vm::AppliedAddressSpaceSwitch {
                prev_tid: 1,
                next_tid: 2,
                active_satp: make_satp(0x22222, 0),
            },
        );

        let sequence = prepare_user_launch_csr_sequence(&frame, scratch, commit).unwrap();

        assert_eq!(sequence.sscratch, scratch.sscratch);
        assert_eq!(sequence.satp, make_satp(0x22222, 0));
        assert!(sequence.sfence_vma_after_satp);
        assert_eq!(sequence.sepc, frame.sepc);
        assert_eq!(sequence.user_sp, frame.sp());
    }

    #[test]
    fn rooted_user_launch_csr_sequence_uses_root_storage_satp() {
        let mut frame = TrapFrame::default();
        frame.sepc = 0x4001_0000;
        frame.set_sp(0x4010_0000);
        let scratch = prepare_enter_user_scratch(&frame).unwrap();
        let root_satp = make_satp(0x22222, 0);
        let commit = prepare_satp_commit(
            make_satp(0x11111, 0),
            super::task_vm::AppliedAddressSpaceSwitch {
                prev_tid: 1,
                next_tid: 2,
                active_satp: root_satp,
            },
        );

        let rooted =
            prepare_rooted_user_launch_csr_sequence(&frame, scratch, commit, root_satp).unwrap();

        assert_eq!(rooted.root_satp, root_satp);
        assert_eq!(rooted.sequence.satp, root_satp);
        assert_eq!(rooted.sequence.sscratch, scratch.sscratch);
        assert_eq!(rooted.sequence.sepc, frame.sepc);
        assert_eq!(rooted.sequence.user_sp, frame.sp());
    }

    #[test]
    fn frame_backed_user_image_plan_uses_pmm_frames_for_entry_and_stack() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut pmm =
            PageFrameManager::<4>::new(0x9000_0000, 0x9000_4000, super::sv39::PAGE_SIZE);

        let plan = plan_frame_backed_user_image(&loaded, &mut pmm, 512).unwrap();
        let segment = plan.segments[0].unwrap();

        assert_eq!(plan.entry_vaddr, loaded.entry);
        assert_eq!(plan.entry_frame_pa, 0x9000_0000);
        assert_eq!(segment.vaddr, 0x4000_0000);
        assert_eq!(segment.frame_pa, 0x9000_0000);
        assert_ne!(segment.frame_pa, segment.vaddr);
        assert_eq!(plan.stack_frame_start_pa, 0x9000_1000);
        assert_eq!(plan.stack_pages, 1);
        assert_eq!(plan.segment_count, loaded.segment_count);
    }

    #[test]
    fn frame_backed_user_image_plan_reports_out_of_frames() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut pmm =
            PageFrameManager::<1>::new(0x9000_0000, 0x9000_1000, super::sv39::PAGE_SIZE);

        assert_eq!(
            plan_frame_backed_user_image(&loaded, &mut pmm, 512),
            Err(UserFrameBackedImageError::OutOfFrames)
        );
    }

    #[test]
    fn frame_backed_user_image_materializes_segments_and_stack_bytes() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut pmm =
            PageFrameManager::<4>::new(0x9000_0000, 0x9000_4000, super::sv39::PAGE_SIZE);
        let plan = plan_frame_backed_user_image(&loaded, &mut pmm, 512).unwrap();
        let mut code_frame = [0u8; super::sv39::PAGE_SIZE];
        let mut stack_frame = [0u8; super::sv39::PAGE_SIZE];
        let mut stack_bytes = [0u8; 512];
        let stack = build_initial_user_stack::<4, 4>(
            &mut stack_bytes,
            loaded.stack_top,
            &["/sbin/init"],
            &["PATH=/bin"],
        )
        .unwrap();
        let mut regions = [FrameRegionMut {
            frame_pa: plan.segments[0].unwrap().frame_pa,
            bytes: &mut code_frame,
        }];

        materialize_frame_backed_user_image(
            &loaded,
            &plan,
            &mut regions,
            &mut stack_frame,
            &stack_bytes,
        )
        .unwrap();

        assert_eq!(&code_frame[..4], &loaded.segments[0].unwrap().data[..4]);
        assert!(code_frame
            .windows(b"[user-init]".len())
            .any(|w| w == b"[user-init]"));
        assert_eq!(&stack_frame[..stack_bytes.len()], &stack_bytes);
        let sp_offset = stack.sp - (loaded.stack_top - stack_bytes.len());
        assert!(sp_offset < stack_bytes.len());
        assert!(stack_frame
            .windows(b"/sbin/init".len())
            .any(|w| w == b"/sbin/init"));
    }

    #[test]
    fn frame_backed_user_image_materialize_rejects_missing_segment_frame() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut pmm =
            PageFrameManager::<4>::new(0x9000_0000, 0x9000_4000, super::sv39::PAGE_SIZE);
        let plan = plan_frame_backed_user_image(&loaded, &mut pmm, 512).unwrap();
        let mut stack_frame = [0u8; super::sv39::PAGE_SIZE];
        let stack_bytes = [0u8; 512];
        let mut regions: [FrameRegionMut<'_>; 0] = [];

        assert_eq!(
            materialize_frame_backed_user_image(
                &loaded,
                &plan,
                &mut regions,
                &mut stack_frame,
                &stack_bytes,
            ),
            Err(FrameBackedMaterializeError::MissingSegmentFrame(
                plan.segments[0].unwrap().frame_pa
            ))
        );
    }

    #[test]
    fn frame_backed_user_image_reuses_frame_for_segments_on_same_page() {
        let elf = build_test_init_elf();
        let mut loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let first = loaded.segments[0].unwrap();
        loaded.segment_count = 2;
        loaded.segments[1] = Some(super::elf_loader::LoadSegment {
            vaddr: first.vaddr + 0x20,
            mem_size: 4,
            file_size: 4,
            flags: 4,
            data: &first.data[..4],
        });
        let mut pmm =
            PageFrameManager::<4>::new(0x9000_0000, 0x9000_4000, super::sv39::PAGE_SIZE);
        let plan = plan_frame_backed_user_image(&loaded, &mut pmm, 512).unwrap();

        assert_eq!(plan.segments[0].unwrap().frame_pa, 0x9000_0000);
        assert_eq!(plan.segments[1].unwrap().frame_pa, 0x9000_0000);
        assert_eq!(plan.stack_frame_start_pa, 0x9000_1000);
    }

    #[test]
    fn frame_backed_user_mapping_installs_4k_leafs_below_root() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut pmm =
            PageFrameManager::<4>::new(0x9000_0000, 0x9000_4000, super::sv39::PAGE_SIZE);
        let plan = plan_frame_backed_user_image(&loaded, &mut pmm, 512).unwrap();
        let mut root = RootPageTable::new();
        let mut l1 = RootPageTable::new();
        let mut l0 = RootPageTable::new();

        let install = install_frame_backed_user_mapping_in_tables(
            &mut root,
            &mut l1,
            0x9000_2000,
            &mut l0,
            0x9000_3000,
            &plan,
        )
        .unwrap();

        let root_entry = root.entries[vpn2(0x4000_0000)];
        let l1_entry = l1.entries[vpn1(0x4000_0000)];
        let code_entry = l0.entries[vpn0(0x4000_0000)];
        let stack_entry = l0.entries[vpn0(0x400f_f000)];

        assert!(is_table_mapping(root_entry));
        assert_eq!(root_entry.ppn(), 0x9000_2000 / super::sv39::PAGE_SIZE);
        assert!(is_table_mapping(l1_entry));
        assert_eq!(l1_entry.ppn(), 0x9000_3000 / super::sv39::PAGE_SIZE);
        assert!(is_leaf_user_mapping(code_entry));
        assert_eq!(code_entry.ppn(), 0x9000_0000 / super::sv39::PAGE_SIZE);
        assert_eq!(code_entry.flags() & PTE_X, PTE_X);
        assert!(is_leaf_user_mapping(stack_entry));
        assert_eq!(stack_entry.ppn(), 0x9000_1000 / super::sv39::PAGE_SIZE);
        assert_eq!(stack_entry.flags() & PTE_W, PTE_W);
        assert_eq!(install.mapped_pages, 2);
    }

    #[test]
    fn frame_backed_user_mapping_merges_flags_for_segments_on_same_page() {
        let elf = build_test_init_elf();
        let mut loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let first = loaded.segments[0].unwrap();
        loaded.segment_count = 2;
        loaded.segments[1] = Some(super::elf_loader::LoadSegment {
            vaddr: first.vaddr + 0x20,
            mem_size: 4,
            file_size: 4,
            flags: 4,
            data: &first.data[..4],
        });
        let mut pmm =
            PageFrameManager::<4>::new(0x9000_0000, 0x9000_4000, super::sv39::PAGE_SIZE);
        let plan = plan_frame_backed_user_image(&loaded, &mut pmm, 512).unwrap();
        let mut root = RootPageTable::new();
        let mut l1 = RootPageTable::new();
        let mut l0 = RootPageTable::new();
        let install = install_frame_backed_user_mapping_in_tables(
            &mut root,
            &mut l1,
            0x9000_2000,
            &mut l0,
            0x9000_3000,
            &plan,
        )
        .unwrap();
        let code_entry = l0.entries[vpn0(first.vaddr)];

        assert_eq!(install.mapped_pages, 2);
        assert_eq!(code_entry.ppn(), 0x9000_0000 / super::sv39::PAGE_SIZE);
        assert_eq!(code_entry.flags() & PTE_X, PTE_X);
        assert_eq!(code_entry.flags() & PTE_R, PTE_R);
    }

    #[test]
    fn frame_backed_user_mapping_rejects_unaligned_table_frames() {
        let elf = build_test_init_elf();
        let loaded = load_user_elf(&elf, 0x4010_0000).unwrap();
        let mut pmm =
            PageFrameManager::<4>::new(0x9000_0000, 0x9000_4000, super::sv39::PAGE_SIZE);
        let plan = plan_frame_backed_user_image(&loaded, &mut pmm, 512).unwrap();
        let mut root = RootPageTable::new();
        let mut l1 = RootPageTable::new();
        let mut l0 = RootPageTable::new();

        assert_eq!(
            install_frame_backed_user_mapping_in_tables(
                &mut root,
                &mut l1,
                0x9000_2001,
                &mut l0,
                0x9000_3000,
                &plan,
            ),
            Err(super::user_vm::UserFrameBackedMappingError::UnalignedTableFrame)
        );
    }

    #[test]
    fn pmm_can_reuse_freed_frame() {
        let mut pmm = PageFrameManager::<8>::new(0x8020_0000, 0x8020_4000, 0x1000);
        let a = pmm.alloc().unwrap();
        let b = pmm.alloc().unwrap();
        assert_eq!(a, 0x8020_0000);
        assert_eq!(b, 0x8020_1000);
        pmm.free(a).unwrap();
        assert_eq!(pmm.alloc(), Some(a));
    }

    #[test]
    fn pmm_rejects_out_of_range_free() {
        let mut pmm = PageFrameManager::<2>::new(0x8020_0000, 0x8020_2000, 0x1000);
        assert_eq!(pmm.free(0x9000_0000), Err(PmmError::OutOfRange));
    }

    #[test]
    fn vm_space_resolves_fault_by_access_mode() {
        let mut vm = VmSpace::<4>::new();
        vm.map_anon(0x4000_0000, 0x4000_1000, PTE_R | PTE_X)
            .unwrap();
        assert!(vm.resolve_fault(0x4000_0004, FaultAccess::Read).is_some());
        assert!(vm
            .resolve_fault(0x4000_0004, FaultAccess::Execute)
            .is_some());
        assert!(vm.resolve_fault(0x4000_0004, FaultAccess::Write).is_none());
    }

    #[test]
    fn vm_space_rejects_invalid_range() {
        let mut vm = VmSpace::<2>::new();
        assert_eq!(
            vm.map_anon(0x1000, 0x1000, PTE_R),
            Err(VmError::InvalidRange)
        );
    }

    #[test]
    fn page_fault_handler_allocates_mapping_for_valid_area() {
        let mut vm = VmSpace::<4>::new();
        vm.map_anon(0x4010_0000, 0x4010_2000, PTE_R | PTE_W)
            .unwrap();
        let mut pmm = PageFrameManager::<4>::new(0x9000_0000, 0x9000_4000, 0x1000);

        let pf = handle_user_page_fault(SCAUSE_STORE_PAGE_FAULT, 0x4010_1234, &vm, &mut pmm);
        match pf {
            PageFaultResult::Mapped(m) => {
                assert_eq!(m.va_page, 0x4010_1000);
                assert_eq!(m.frame_pa, 0x9000_0000);
                assert_ne!(m.pte_flags & PTE_W, 0);
            }
            _ => panic!("expected mapped fault"),
        }
    }

    #[test]
    fn page_fault_handler_rejects_illegal_access() {
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0x4000_0000, 0x4000_1000, PTE_R | PTE_X)
            .unwrap();
        let mut pmm = PageFrameManager::<2>::new(0x9000_0000, 0x9000_2000, 0x1000);

        let pf = handle_user_page_fault(SCAUSE_STORE_PAGE_FAULT, 0x4000_0010, &vm, &mut pmm);
        assert_eq!(pf, PageFaultResult::InvalidAccess);
    }

    #[test]
    fn page_fault_handler_load_fault_maps_readable_area() {
        let mut vm = VmSpace::<2>::new();
        vm.map_anon(0x8060_0000, 0x8060_1000, PTE_R).unwrap();
        let mut pmm = PageFrameManager::<2>::new(0x9000_0000, 0x9000_2000, 0x1000);

        let pf = handle_user_page_fault(SCAUSE_LOAD_PAGE_FAULT, 0x8060_00f0, &vm, &mut pmm);
        assert!(matches!(pf, PageFaultResult::Mapped(_)));
    }

    fn build_test_ext4_image() -> Vec<u8> {
        let elf = build_test_init_elf();
        let mut image = vec![0u8; super::demo_user_init::DEMO_EXT4_IMAGE_SIZE];
        super::demo_user_init::build_demo_ext4_image(&mut image, &elf).unwrap();
        image
    }

    fn build_test_init_elf() -> Vec<u8> {
        let mut elf = vec![0u8; super::demo_user_init::DEMO_INIT_ELF_SIZE];
        let len = super::demo_user_init::build_demo_init_elf(&mut elf).unwrap();
        elf.truncate(len);
        elf
    }

    fn write_cstr_at(buf: &mut [u8], offset: usize, bytes: &[u8]) {
        buf[offset..offset + bytes.len()].copy_from_slice(bytes);
        buf[offset + bytes.len()] = 0;
    }

    fn write_usize_at(buf: &mut [u8], offset: usize, value: usize) {
        let bytes = value.to_ne_bytes();
        let word = core::mem::size_of::<usize>();
        buf[offset..offset + word].copy_from_slice(&bytes);
    }

    fn read_usize_at(buf: &[u8], offset: usize) -> usize {
        let mut bytes = [0u8; core::mem::size_of::<usize>()];
        bytes.copy_from_slice(&buf[offset..offset + core::mem::size_of::<usize>()]);
        usize::from_le_bytes(bytes)
    }

    fn read_cstr_at<'a>(buf: &'a [u8], base_va: usize, ptr_va: usize) -> &'a [u8] {
        let mut off = ptr_va - base_va;
        let start = off;
        while off < buf.len() && buf[off] != 0 {
            off += 1;
        }
        &buf[start..off]
    }
}
