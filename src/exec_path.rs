// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::elf_loader::InitLoadError;
use crate::exec_args::{ExecvArgBlock, ExecvArgError};
use crate::user_exec::{load_init_exec_image, UserExecImage};
use crate::vfs::Vfs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecPathError {
    Args(ExecvArgError),
    Load(InitLoadError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecPathImageReport {
    pub path_len: usize,
    pub argc: usize,
    pub entry: usize,
    pub stack_top: usize,
    pub segment_count: usize,
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
