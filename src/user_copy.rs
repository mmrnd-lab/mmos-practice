// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::vm::{FaultAccess, VmSpace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserCopyError {
    BadAddress,
    BufferTooSmall,
    MissingNul,
    Permission,
}

pub struct UserSlice<'a> {
    bytes: &'a [u8],
    base: usize,
}

impl<'a> UserSlice<'a> {
    pub const fn new(bytes: &'a [u8], base: usize) -> Self {
        Self { bytes, base }
    }

    fn range(&self, ptr: usize, len: usize) -> Result<&'a [u8], UserCopyError> {
        let start = ptr.checked_sub(self.base).ok_or(UserCopyError::BadAddress)?;
        let end = start.checked_add(len).ok_or(UserCopyError::BadAddress)?;
        self.bytes.get(start..end).ok_or(UserCopyError::BadAddress)
    }

    pub fn copyin(&self, ptr: usize, out: &mut [u8]) -> Result<usize, UserCopyError> {
        let src = self.range(ptr, out.len())?;
        out.copy_from_slice(src);
        Ok(out.len())
    }

    pub fn copyin_bytes(
        &self,
        ptr: usize,
        len: usize,
        out: &mut [u8],
    ) -> Result<usize, UserCopyError> {
        if len > out.len() {
            return Err(UserCopyError::BufferTooSmall);
        }
        let src = self.range(ptr, len)?;
        out[..len].copy_from_slice(src);
        Ok(len)
    }

    pub fn copyin_cstr<'b>(
        &self,
        ptr: usize,
        out: &'b mut [u8],
    ) -> Result<&'b str, UserCopyError> {
        if out.is_empty() {
            return Err(UserCopyError::BufferTooSmall);
        }

        let mut copied = 0usize;
        while copied < out.len() {
            let byte = self.range(ptr + copied, 1)?[0];
            if byte == 0 {
                return core::str::from_utf8(&out[..copied]).map_err(|_| UserCopyError::BadAddress);
            }
            out[copied] = byte;
            copied += 1;
        }
        Err(UserCopyError::MissingNul)
    }
}


pub struct CheckedUserSlice<'a, const MAX_AREAS: usize> {
    user: UserSlice<'a>,
    vm: &'a VmSpace<MAX_AREAS>,
}

impl<'a, const MAX_AREAS: usize> CheckedUserSlice<'a, MAX_AREAS> {
    pub const fn new(bytes: &'a [u8], base: usize, vm: &'a VmSpace<MAX_AREAS>) -> Self {
        Self {
            user: UserSlice::new(bytes, base),
            vm,
        }
    }

    fn check_read(&self, ptr: usize, len: usize) -> Result<(), UserCopyError> {
        if self.vm.allows_range(ptr, len, FaultAccess::Read) {
            Ok(())
        } else {
            Err(UserCopyError::Permission)
        }
    }

    pub fn copyin_bytes(
        &self,
        ptr: usize,
        len: usize,
        out: &mut [u8],
    ) -> Result<usize, UserCopyError> {
        self.check_read(ptr, len)?;
        self.user.copyin_bytes(ptr, len, out)
    }

    pub fn copyin_cstr<'b>(
        &self,
        ptr: usize,
        out: &'b mut [u8],
    ) -> Result<&'b str, UserCopyError> {
        if out.is_empty() {
            return Err(UserCopyError::BufferTooSmall);
        }

        let mut copied = 0usize;
        while copied < out.len() {
            self.check_read(ptr + copied, 1)?;
            let mut byte = [0u8; 1];
            self.user.copyin(ptr + copied, &mut byte)?;
            if byte[0] == 0 {
                return core::str::from_utf8(&out[..copied])
                    .map_err(|_| UserCopyError::BadAddress);
            }
            out[copied] = byte[0];
            copied += 1;
        }
        Err(UserCopyError::MissingNul)
    }
}


pub struct UserSliceMut<'a> {
    bytes: &'a mut [u8],
    base: usize,
}

impl<'a> UserSliceMut<'a> {
    pub fn new(bytes: &'a mut [u8], base: usize) -> Self {
        Self { bytes, base }
    }

    fn range_mut(&mut self, ptr: usize, len: usize) -> Result<&mut [u8], UserCopyError> {
        let start = ptr.checked_sub(self.base).ok_or(UserCopyError::BadAddress)?;
        let end = start.checked_add(len).ok_or(UserCopyError::BadAddress)?;
        self.bytes.get_mut(start..end).ok_or(UserCopyError::BadAddress)
    }

    pub fn copyout_bytes(&mut self, ptr: usize, src: &[u8]) -> Result<usize, UserCopyError> {
        let dst = self.range_mut(ptr, src.len())?;
        dst.copy_from_slice(src);
        Ok(src.len())
    }
}

pub struct CheckedUserSliceMut<'a, const MAX_AREAS: usize> {
    user: UserSliceMut<'a>,
    vm: &'a VmSpace<MAX_AREAS>,
}

impl<'a, const MAX_AREAS: usize> CheckedUserSliceMut<'a, MAX_AREAS> {
    pub fn new(bytes: &'a mut [u8], base: usize, vm: &'a VmSpace<MAX_AREAS>) -> Self {
        Self {
            user: UserSliceMut::new(bytes, base),
            vm,
        }
    }

    fn check_write(&self, ptr: usize, len: usize) -> Result<(), UserCopyError> {
        if self.vm.allows_range(ptr, len, FaultAccess::Write) {
            Ok(())
        } else {
            Err(UserCopyError::Permission)
        }
    }

    pub fn copyout_bytes(&mut self, ptr: usize, src: &[u8]) -> Result<usize, UserCopyError> {
        self.check_write(ptr, src.len())?;
        self.user.copyout_bytes(ptr, src)
    }
}
