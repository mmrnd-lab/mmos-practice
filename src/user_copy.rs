// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserCopyError {
    BadAddress,
    BufferTooSmall,
    MissingNul,
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
