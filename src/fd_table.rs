// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::char_device::{CharError, CharacterDevice};
use crate::devfs::{DeviceNamespace, VfsLookupError, VfsNode, VfsNodeKind, VfsNodeTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileError {
    BadFd,
    NotReadable,
    NotWritable,
    NotCharacterDevice,
    Lookup(VfsLookupError),
    Char(CharError),
}

impl From<CharError> for FileError {
    fn from(value: CharError) -> Self {
        Self::Char(value)
    }
}

impl From<VfsLookupError> for FileError {
    fn from(value: VfsLookupError) -> Self {
        Self::Lookup(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

impl OpenMode {
    pub const fn can_read(self) -> bool {
        matches!(self, Self::ReadOnly | Self::ReadWrite)
    }

    pub const fn can_write(self) -> bool {
        matches!(self, Self::WriteOnly | Self::ReadWrite)
    }
}

#[derive(Clone, Copy)]
pub enum FileTarget {
    Character(&'static dyn CharacterDevice),
}

#[derive(Clone, Copy)]
pub struct OpenFile {
    target: FileTarget,
    mode: OpenMode,
    offset: usize,
}

impl OpenFile {
    pub const fn new(target: FileTarget, mode: OpenMode) -> Self {
        Self {
            target,
            mode,
            offset: 0,
        }
    }

    pub const fn read_only_character(device: &'static dyn CharacterDevice) -> Self {
        Self::new(FileTarget::Character(device), OpenMode::ReadOnly)
    }

    pub const fn write_only_character(device: &'static dyn CharacterDevice) -> Self {
        Self::new(FileTarget::Character(device), OpenMode::WriteOnly)
    }

    pub const fn read_write_character(device: &'static dyn CharacterDevice) -> Self {
        Self::new(FileTarget::Character(device), OpenMode::ReadWrite)
    }

    pub fn from_vfs_node(node: VfsNode<'_>, mode: OpenMode) -> Result<Self, FileError> {
        if node.kind() != VfsNodeKind::CharacterDevice {
            return Err(FileError::NotCharacterDevice);
        }
        match node.target() {
            VfsNodeTarget::Character(device) => Ok(Self::new(FileTarget::Character(device), mode)),
            _ => Err(FileError::NotCharacterDevice),
        }
    }

    pub const fn offset(&self) -> usize {
        self.offset
    }

    pub fn read(&self, out: &mut [u8]) -> Result<usize, FileError> {
        if !self.mode.can_read() {
            return Err(FileError::NotReadable);
        }
        match self.target {
            FileTarget::Character(device) => device.read(out).map_err(FileError::from),
        }
    }

    pub fn write(&self, bytes: &[u8]) -> Result<usize, FileError> {
        if !self.mode.can_write() {
            return Err(FileError::NotWritable);
        }
        match self.target {
            FileTarget::Character(device) => device.write(bytes).map_err(FileError::from),
        }
    }
}

pub struct FileDescriptorTable<const N: usize> {
    slots: [Option<OpenFile>; N],
}

impl<const N: usize> FileDescriptorTable<N> {
    pub const fn new() -> Self {
        Self { slots: [None; N] }
    }

    pub fn get(&self, fd: usize) -> Result<&OpenFile, FileError> {
        self.slots
            .get(fd)
            .and_then(Option::as_ref)
            .ok_or(FileError::BadFd)
    }

    pub fn preopen(&mut self, fd: usize, file: OpenFile) -> Result<(), FileError> {
        match self.slots.get_mut(fd) {
            Some(slot) => {
                *slot = Some(file);
                Ok(())
            }
            None => Err(FileError::BadFd),
        }
    }

    pub fn allocate(&mut self, file: OpenFile) -> Result<usize, FileError> {
        for (fd, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(file);
                return Ok(fd);
            }
        }
        Err(FileError::BadFd)
    }

    pub fn read(&self, fd: usize, out: &mut [u8]) -> Result<usize, FileError> {
        self.get(fd)?.read(out)
    }

    pub fn write(&self, fd: usize, bytes: &[u8]) -> Result<usize, FileError> {
        self.get(fd)?.write(bytes)
    }

    pub fn open_path(
        &mut self,
        ns: &DeviceNamespace<'_>,
        path: &str,
        mode: OpenMode,
    ) -> Result<usize, FileError> {
        let node = ns.lookup_path(path)?;
        let file = OpenFile::from_vfs_node(node, mode)?;
        self.allocate(file)
    }
}

pub fn stdio_table(console: &'static dyn CharacterDevice) -> FileDescriptorTable<3> {
    stdio_table_with_capacity(console)
}

pub fn stdio_table_with_capacity<const N: usize>(
    console: &'static dyn CharacterDevice,
) -> FileDescriptorTable<N> {
    let mut table = FileDescriptorTable::new();
    let _ = table.preopen(0, OpenFile::read_only_character(console));
    let _ = table.preopen(1, OpenFile::write_only_character(console));
    let _ = table.preopen(2, OpenFile::write_only_character(console));
    table
}
