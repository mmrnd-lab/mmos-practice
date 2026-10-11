// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::elf_loader::LoadedUserImage;
use crate::trap::TrapFrame;
use crate::user_exec::{install_exec_context, UserExecImage};
use crate::user_stack::{build_initial_user_stack, UserStackError};
use crate::user_vm::{build_user_vm_plan, UserVmError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserLaunchError {
    Vm(UserVmError),
    Stack(UserStackError),
}

impl From<UserVmError> for UserLaunchError {
    fn from(value: UserVmError) -> Self {
        Self::Vm(value)
    }
}

impl From<UserStackError> for UserLaunchError {
    fn from(value: UserStackError) -> Self {
        Self::Stack(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UserLaunchPlan {
    pub trapframe: TrapFrame,
    pub satp: usize,
    pub mapped_regions: usize,
    pub sp: usize,
    pub argc: usize,
    pub argv_ptr: usize,
    pub envp_ptr: usize,
}

pub fn prepare_user_init_launch<const MAX_ARGS: usize, const MAX_ENVS: usize>(
    loaded: &LoadedUserImage<'_>,
    root_frame_pa: usize,
    stack_bytes: &mut [u8],
    argv: &[&str],
    envp: &[&str],
) -> Result<UserLaunchPlan, UserLaunchError> {
    let vm_plan = build_user_vm_plan(loaded, root_frame_pa)?;
    let image = UserExecImage::from_loaded(loaded);

    let mut trapframe = TrapFrame::default();
    install_exec_context(&mut trapframe, &image);

    let stack =
        build_initial_user_stack::<MAX_ARGS, MAX_ENVS>(stack_bytes, loaded.stack_top, argv, envp)?;
    trapframe.set_sp(stack.sp);
    trapframe.set_a0(stack.argc);
    trapframe.set_a1(stack.argv_ptr);
    trapframe.set_a2(stack.envp_ptr);

    Ok(UserLaunchPlan {
        trapframe,
        satp: vm_plan.satp,
        mapped_regions: vm_plan.mapped_regions,
        sp: stack.sp,
        argc: stack.argc,
        argv_ptr: stack.argv_ptr,
        envp_ptr: stack.envp_ptr,
    })
}
