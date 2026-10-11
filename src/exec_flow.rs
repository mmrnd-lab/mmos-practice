// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::exec_args::{
    copyin_execv_args, materialize_execv_stack, ExecvArgBlock, ExecvArgError,
};
use crate::exec_path::{load_exec_image_from_args, ExecPathError};
use crate::process_model::ProcError;
use crate::trap_runtime::{ProcessExecRuntime, ProcessTrapRuntime};
use crate::user_copy::CheckedUserSlice;
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
    vfs: &Vfs<'_>,
    scratch: &mut [u8],
    stack_bytes: &mut [u8],
    stack_top_va: usize,
    new_vm: VmSpace<MAX_AREAS>,
) -> Result<(ProcessExecRuntime, ExecvApplyReport), ExecvApplyError> {
    let mut block = ExecvArgBlock::<MAX_ARGS, ARG_BYTES>::new();
    let args = copyin_execv_args(user, path_ptr, argv_ptr, &mut block)
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
        entry: exec.entry,
        image_stack_top: loaded.stack_top,
        initial_sp: layout.sp,
        segment_count: loaded.segment_count,
        pid: exec.pid,
        tid: exec.tid,
    };
    Ok((exec, report))
}
