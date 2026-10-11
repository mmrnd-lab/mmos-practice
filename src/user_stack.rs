// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStackError {
    TooManyArgs,
    TooManyEnvs,
    StackOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InitialUserStack {
    pub sp: usize,
    pub argc: usize,
    pub argv_ptr: usize,
    pub envp_ptr: usize,
}

pub fn build_initial_user_stack<const MAX_ARGS: usize, const MAX_ENVS: usize>(
    stack_bytes: &mut [u8],
    stack_top_va: usize,
    argv: &[&str],
    envp: &[&str],
) -> Result<InitialUserStack, UserStackError> {
    if argv.len() > MAX_ARGS {
        return Err(UserStackError::TooManyArgs);
    }
    if envp.len() > MAX_ENVS {
        return Err(UserStackError::TooManyEnvs);
    }

    let mut cursor = stack_bytes.len();
    let mut argv_addrs = [0usize; MAX_ARGS];
    let mut envp_addrs = [0usize; MAX_ENVS];

    let mut i = argv.len();
    while i > 0 {
        i -= 1;
        argv_addrs[i] = push_cstr(stack_bytes, stack_top_va, &mut cursor, argv[i].as_bytes())?;
    }

    let mut j = envp.len();
    while j > 0 {
        j -= 1;
        envp_addrs[j] = push_cstr(stack_bytes, stack_top_va, &mut cursor, envp[j].as_bytes())?;
    }

    let word = core::mem::size_of::<usize>();
    let total_words = 1 + argv.len() + 1 + envp.len() + 1;
    let table_bytes = total_words * word;
    let aligned = cursor & !(word - 1);
    if aligned < table_bytes {
        return Err(UserStackError::StackOverflow);
    }
    cursor = aligned - table_bytes;

    let mut off = cursor;
    write_word(stack_bytes, &mut off, argv.len())?;

    let argv_ptr = stack_top_va - (stack_bytes.len() - off);
    let mut a = 0usize;
    while a < argv.len() {
        write_word(stack_bytes, &mut off, argv_addrs[a])?;
        a += 1;
    }
    write_word(stack_bytes, &mut off, 0)?;

    let envp_ptr = stack_top_va - (stack_bytes.len() - off);
    let mut e = 0usize;
    while e < envp.len() {
        write_word(stack_bytes, &mut off, envp_addrs[e])?;
        e += 1;
    }
    write_word(stack_bytes, &mut off, 0)?;

    let sp = stack_top_va - (stack_bytes.len() - cursor);
    Ok(InitialUserStack {
        sp,
        argc: argv.len(),
        argv_ptr,
        envp_ptr,
    })
}

fn push_cstr(
    stack_bytes: &mut [u8],
    stack_top_va: usize,
    cursor: &mut usize,
    bytes: &[u8],
) -> Result<usize, UserStackError> {
    let need = bytes.len() + 1;
    if *cursor < need {
        return Err(UserStackError::StackOverflow);
    }
    *cursor -= need;
    let start = *cursor;
    stack_bytes[start..start + bytes.len()].copy_from_slice(bytes);
    stack_bytes[start + bytes.len()] = 0;
    Ok(stack_top_va - (stack_bytes.len() - start))
}

fn write_word(stack_bytes: &mut [u8], off: &mut usize, value: usize) -> Result<(), UserStackError> {
    let word = core::mem::size_of::<usize>();
    if *off + word > stack_bytes.len() {
        return Err(UserStackError::StackOverflow);
    }
    stack_bytes[*off..*off + word].copy_from_slice(&value.to_le_bytes());
    *off += word;
    Ok(())
}
