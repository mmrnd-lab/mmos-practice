// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::elf_loader::InitLoadError;
use crate::exec_args::{ExecvArgBlock, ExecvArgError};
use crate::user_exec::{load_init_exec_image, UserExecImage};
use crate::user_vm::{
    load_shared_object_dependency_in_tables, SharedObjectDependency, SharedObjectLoadPipeline,
    SharedObjectPathLoadError,
};
use crate::vfs::Vfs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecPathError {
    Args(ExecvArgError),
    Load(InitLoadError),
    SharedObject(SharedObjectPathLoadError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecPathImageReport {
    pub path_len: usize,
    pub argc: usize,
    pub entry: usize,
    pub stack_top: usize,
    pub segment_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecDependencyLoadReport {
    pub exec: ExecPathImageReport,
    pub dependency_path: &'static str,
    pub dependency_base: usize,
    pub dependency_entry: usize,
    pub dependency_mapped_pages: usize,
}

pub fn load_exec_image_from_args<
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
>(
    vfs: &Vfs<'_>,
    block: &ExecvArgBlock<MAX_ARGS, ARG_BYTES>,
    scratch: &mut [u8],
    stack_top: usize,
) -> Result<(UserExecImage, ExecPathImageReport), ExecPathError> {
    let path = block.path_str().map_err(ExecPathError::Args)?;
    let loaded = load_init_exec_image(vfs, path, scratch, stack_top).map_err(ExecPathError::Load)?;
    let image = UserExecImage::from_loaded(&loaded);
    let report = ExecPathImageReport {
        path_len: block.path_len,
        argc: block.argc,
        entry: image.entry,
        stack_top: image.stack_top,
        segment_count: image.segment_count,
    };
    Ok((image, report))
}

pub fn load_exec_image_with_shared_object_dependency_from_args<
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
    const FREE_CAP: usize,
>(
    vfs: &Vfs<'_>,
    block: &ExecvArgBlock<MAX_ARGS, ARG_BYTES>,
    exec_scratch: &mut [u8],
    shared_scratch: &mut [u8],
    stack_top: usize,
    dependency: SharedObjectDependency,
    pmm: &mut crate::pmm::PageFrameManager<FREE_CAP>,
    root: &mut crate::sv39::RootPageTable,
    l1: &mut crate::sv39::RootPageTable,
    l1_frame_pa: usize,
    l0: &mut crate::sv39::RootPageTable,
    l0_frame_pa: usize,
) -> Result<(UserExecImage, SharedObjectLoadPipeline, ExecDependencyLoadReport), ExecPathError> {
    let (image, exec_report) =
        load_exec_image_from_args(vfs, block, exec_scratch, stack_top)?;
    let shared = load_shared_object_dependency_in_tables(
        vfs,
        dependency,
        shared_scratch,
        pmm,
        root,
        l1,
        l1_frame_pa,
        l0,
        l0_frame_pa,
    )
    .map_err(ExecPathError::SharedObject)?;
    let report = ExecDependencyLoadReport {
        exec: exec_report,
        dependency_path: dependency.path,
        dependency_base: shared.base,
        dependency_entry: shared.entry_vaddr,
        dependency_mapped_pages: shared.mapped_pages,
    };
    Ok((image, shared, report))
}
