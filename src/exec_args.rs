// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::user_copy::{CheckedUserSlice, UserCopyError};
use crate::user_stack::{InitialUserStack, UserStackError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecvArgError {
    Copy(UserCopyError),
    Stack(UserStackError),
    TooManyArgs,
    MissingArgvNull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecvArgReport {
    pub path_ptr: usize,
    pub argv_ptr: usize,
    pub path_len: usize,
    pub argc: usize,
    pub first_arg_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecvArgBlock<const MAX_ARGS: usize, const ARG_BYTES: usize> {
    pub path: [u8; ARG_BYTES],
    pub path_len: usize,
    pub args: [[u8; ARG_BYTES]; MAX_ARGS],
    pub arg_lens: [usize; MAX_ARGS],
    pub argc: usize,
}

impl<const MAX_ARGS: usize, const ARG_BYTES: usize> ExecvArgBlock<MAX_ARGS, ARG_BYTES> {
    pub const fn new() -> Self {
        Self {
            path: [0; ARG_BYTES],
            path_len: 0,
            args: [[0; ARG_BYTES]; MAX_ARGS],
            arg_lens: [0; MAX_ARGS],
            argc: 0,
        }
    }

    pub fn path_str(&self) -> Result<&str, ExecvArgError> {
        core::str::from_utf8(&self.path[..self.path_len])
            .map_err(|_| ExecvArgError::Copy(UserCopyError::BadAddress))
    }

    pub fn arg_str(&self, idx: usize) -> Result<&str, ExecvArgError> {
        if idx >= self.argc {
            return Err(ExecvArgError::Copy(UserCopyError::BadAddress));
        }
        core::str::from_utf8(&self.args[idx][..self.arg_lens[idx]])
            .map_err(|_| ExecvArgError::Copy(UserCopyError::BadAddress))
    }
}

pub fn copyin_execv_args<
    const A: usize,
    const MAX_ARGS: usize,
    const ARG_BYTES: usize,
>(
    user: &CheckedUserSlice<'_, A>,
    path_ptr: usize,
    argv_ptr: usize,
    block: &mut ExecvArgBlock<MAX_ARGS, ARG_BYTES>,
) -> Result<ExecvArgReport, ExecvArgError> {
    let path_len = {
        let path = user
            .copyin_cstr(path_ptr, &mut block.path)
            .map_err(ExecvArgError::Copy)?;
        path.len()
    };
    block.path_len = path_len;

    let mut argc = 0usize;
    while argc <= MAX_ARGS {
        let arg_ptr = read_user_usize(user, argv_ptr + argc * core::mem::size_of::<usize>())?;
        if arg_ptr == 0 {
            block.argc = argc;
            return Ok(ExecvArgReport {
                path_ptr,
                argv_ptr,
                path_len,
                argc,
                first_arg_len: if argc == 0 { 0 } else { block.arg_lens[0] },
            });
        }
        if argc == MAX_ARGS {
            return Err(ExecvArgError::TooManyArgs);
        }

        let arg_len = {
            let arg = user
                .copyin_cstr(arg_ptr, &mut block.args[argc])
                .map_err(ExecvArgError::Copy)?;
            arg.len()
        };
        block.arg_lens[argc] = arg_len;
        argc += 1;
    }

    Err(ExecvArgError::MissingArgvNull)
}

pub fn materialize_execv_stack<const MAX_ARGS: usize, const ARG_BYTES: usize>(
    block: &ExecvArgBlock<MAX_ARGS, ARG_BYTES>,
    stack_bytes: &mut [u8],
    stack_top_va: usize,
) -> Result<InitialUserStack, ExecvArgError> {
    let mut cursor = stack_bytes.len();
    let mut argv_addrs = [0usize; MAX_ARGS];

    let mut idx = block.argc;
    while idx > 0 {
        idx -= 1;
        argv_addrs[idx] = push_cstr_bytes(
            stack_bytes,
            stack_top_va,
            &mut cursor,
            &block.args[idx][..block.arg_lens[idx]],
        )?;
    }

    let word = core::mem::size_of::<usize>();
    let total_words = 1 + block.argc + 1 + 1;
    let table_bytes = total_words * word;
    let aligned = cursor & !(word - 1);
    if aligned < table_bytes {
        return Err(ExecvArgError::Stack(UserStackError::StackOverflow));
    }
    cursor = aligned - table_bytes;

    let mut off = cursor;
    write_word(stack_bytes, &mut off, block.argc)?;
    let argv_ptr = stack_top_va - (stack_bytes.len() - off);
    let mut arg_idx = 0usize;
    while arg_idx < block.argc {
        write_word(stack_bytes, &mut off, argv_addrs[arg_idx])?;
        arg_idx += 1;
    }
    write_word(stack_bytes, &mut off, 0)?;
    let envp_ptr = stack_top_va - (stack_bytes.len() - off);
    write_word(stack_bytes, &mut off, 0)?;

    Ok(InitialUserStack {
        sp: stack_top_va - (stack_bytes.len() - cursor),
        argc: block.argc,
        argv_ptr,
        envp_ptr,
    })
}

fn read_user_usize<const A: usize>(
    user: &CheckedUserSlice<'_, A>,
    ptr: usize,
) -> Result<usize, ExecvArgError> {
    let mut raw = [0u8; core::mem::size_of::<usize>()];
    user.copyin_bytes(ptr, raw.len(), &mut raw)
        .map_err(ExecvArgError::Copy)?;
    Ok(usize::from_ne_bytes(raw))
}

fn push_cstr_bytes(
    stack_bytes: &mut [u8],
    stack_top_va: usize,
    cursor: &mut usize,
    bytes: &[u8],
) -> Result<usize, ExecvArgError> {
    let need = bytes.len() + 1;
    if *cursor < need {
        return Err(ExecvArgError::Stack(UserStackError::StackOverflow));
    }
    let start = *cursor - need;
    stack_bytes[start..start + bytes.len()].copy_from_slice(bytes);
    stack_bytes[start + bytes.len()] = 0;
    *cursor = start;
    Ok(stack_top_va - (stack_bytes.len() - start))
}

fn write_word(
    stack_bytes: &mut [u8],
    off: &mut usize,
    value: usize,
) -> Result<(), ExecvArgError> {
    let word = core::mem::size_of::<usize>();
    if *off + word > stack_bytes.len() {
        return Err(ExecvArgError::Stack(UserStackError::StackOverflow));
    }
    stack_bytes[*off..*off + word].copy_from_slice(&value.to_ne_bytes());
    *off += word;
    Ok(())
}
