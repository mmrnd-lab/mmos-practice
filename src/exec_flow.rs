// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::exec_args::{
    copyin_execv_args, materialize_execv_stack, ExecvArgBlock, ExecvArgError,
};
use crate::exec_path::{
    load_exec_image_from_args, load_exec_image_with_shared_object_dependency_from_args,
    ExecPathError,
};
use crate::process_model::ProcError;
use crate::trap_runtime::{ProcessExecRuntime, ProcessSharedObjectRuntime, ProcessTrapRuntime};
use crate::user_copy::CheckedUserSlice;
use crate::user_vm::SharedObjectDependency;
use crate::vfs::Vfs;
use crate::vm::VmSpace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecvApplyError {
    Args(ExecvArgError),
    Path(ExecPathError),
    Runtime(ProcError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecvApplyReport {
    pub path_len: usize,
    pub argc: usize,
    pub envc: usize,
    pub entry: usize,
    pub image_stack_top: usize,
    pub initial_sp: usize,
    pub segment_count: usize,
    pub pid: usize,
    pub tid: usize,
}

pub fn apply_execv_from_user_args<
    const USER_AREAS: usize,
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
>(
    runtime: &mut ProcessTrapRuntime<MAX_PROCS, MAX_CHILDREN, MAX_AREAS, MAX_TASKS>,
    tid: usize,
    user: &CheckedUserSlice<'_, USER_AREAS>,
    path_ptr: usize,
    argv_ptr: usize,
    envp_ptr: usize,
    vfs: &Vfs<'_>,
    scratch: &mut [u8],
    stack_bytes: &mut [u8],
    stack_top_va: usize,
    new_vm: VmSpace<MAX_AREAS>,
) -> Result<(ProcessExecRuntime, ExecvApplyReport), ExecvApplyError> {
    let mut block = ExecvArgBlock::<MAX_ARGS, ARG_BYTES>::new();
    let args = copyin_execv_args(user, path_ptr, argv_ptr, envp_ptr, &mut block)
        .map_err(ExecvApplyError::Args)?;

    let layout = materialize_execv_stack(&block, stack_bytes, stack_top_va)
        .map_err(ExecvApplyError::Args)?;

    let (mut image, loaded) = load_exec_image_from_args(vfs, &block, scratch, stack_top_va)
        .map_err(ExecvApplyError::Path)?;
    image.stack_top = layout.sp;

    let exec = runtime
        .exec_task_image(tid, image, new_vm)
        .map_err(ExecvApplyError::Runtime)?;

    let report = ExecvApplyReport {
        path_len: args.path_len,
        argc: layout.argc,
        envc: args.envc,
        entry: exec.entry,
        image_stack_top: loaded.stack_top,
        initial_sp: layout.sp,
        segment_count: loaded.segment_count,
        pid: exec.pid,
        tid: exec.tid,
    };
    Ok((exec, report))
}

pub fn apply_execv_with_shared_object_dependency_from_user_args<
    const USER_AREAS: usize,
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const MAX_PROCS: usize,
    const MAX_CHILDREN: usize,
    const MAX_AREAS: usize,
    const MAX_TASKS: usize,
    const FREE_CAP: usize,
>(
    runtime: &mut ProcessTrapRuntime<MAX_PROCS, MAX_CHILDREN, MAX_AREAS, MAX_TASKS>,
    tid: usize,
    user: &CheckedUserSlice<'_, USER_AREAS>,
    path_ptr: usize,
    argv_ptr: usize,
    envp_ptr: usize,
    vfs: &Vfs<'_>,
    exec_scratch: &mut [u8],
    shared_scratch: &mut [u8],
    stack_bytes: &mut [u8],
    stack_top_va: usize,
    new_vm: VmSpace<MAX_AREAS>,
    dependency: SharedObjectDependency,
    pmm: &mut crate::pmm::PageFrameManager<FREE_CAP>,
    root: &mut crate::sv39::RootPageTable,
    l1: &mut crate::sv39::RootPageTable,
    l1_frame_pa: usize,
    l0: &mut crate::sv39::RootPageTable,
    l0_frame_pa: usize,
) -> Result<(ProcessExecRuntime, ExecvApplyReport), ExecvApplyError> {
    let mut block = ExecvArgBlock::<MAX_ARGS, ARG_BYTES>::new();
    let args = copyin_execv_args(user, path_ptr, argv_ptr, envp_ptr, &mut block)
        .map_err(ExecvApplyError::Args)?;

    let layout = materialize_execv_stack(&block, stack_bytes, stack_top_va)
        .map_err(ExecvApplyError::Args)?;

    let (mut image, shared, loaded) = load_exec_image_with_shared_object_dependency_from_args(
        vfs,
        &block,
        exec_scratch,
        shared_scratch,
        stack_top_va,
        dependency,
        pmm,
        root,
        l1,
        l1_frame_pa,
        l0,
        l0_frame_pa,
    )
    .map_err(ExecvApplyError::Path)?;
    image.stack_top = layout.sp;

    let shared_object = ProcessSharedObjectRuntime {
        path: loaded.dependency_path,
        base: loaded.dependency_base,
        entry: loaded.dependency_entry,
        mapped_pages: shared.mapped_pages,
    };
    let exec = runtime
        .exec_task_image_with_shared_object(tid, image, new_vm, Some(shared_object))
        .map_err(ExecvApplyError::Runtime)?;

    let report = ExecvApplyReport {
        path_len: args.path_len,
        argc: layout.argc,
        envc: args.envc,
        entry: exec.entry,
        image_stack_top: loaded.exec.stack_top,
        initial_sp: layout.sp,
        segment_count: loaded.exec.segment_count,
        pid: exec.pid,
        tid: exec.tid,
    };
    Ok((exec, report))
}
