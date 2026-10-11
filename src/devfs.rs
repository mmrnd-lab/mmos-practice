// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

use crate::block::BlockDevice;
use crate::char_device::CharacterDevice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsLookupError {
    BadPath,
    NotDirectory,
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsNodeKind {
    Directory,
    CharacterDevice,
    BlockDevice,
}

#[derive(Clone, Copy)]
pub enum VfsNodeTarget<'a> {
    None,
    Character(&'static dyn CharacterDevice),
    Block(&'a dyn BlockDevice),
}

#[derive(Clone, Copy)]
pub struct VfsNode<'a> {
    path: &'static str,
    name: &'static str,
    kind: VfsNodeKind,
    target: VfsNodeTarget<'a>,
}

impl<'a> VfsNode<'a> {
    pub const fn directory(path: &'static str, name: &'static str) -> Self {
        Self {
            path,
            name,
            kind: VfsNodeKind::Directory,
            target: VfsNodeTarget::None,
        }
    }

    pub const fn character(
        path: &'static str,
        name: &'static str,
        device: &'static dyn CharacterDevice,
    ) -> Self {
        Self {
            path,
            name,
            kind: VfsNodeKind::CharacterDevice,
            target: VfsNodeTarget::Character(device),
        }
    }

    pub const fn block(
        path: &'static str,
        name: &'static str,
        device: &'a dyn BlockDevice,
    ) -> Self {
        Self {
            path,
            name,
            kind: VfsNodeKind::BlockDevice,
            target: VfsNodeTarget::Block(device),
        }
    }

    pub const fn path(&self) -> &str {
        self.path
    }

    pub const fn name(&self) -> &str {
        self.name
    }

    pub const fn kind(&self) -> VfsNodeKind {
        self.kind
    }

    pub const fn target(&self) -> VfsNodeTarget<'a> {
        self.target
    }
}

pub struct DeviceNamespace<'a> {
    console: &'static dyn CharacterDevice,
    block0: Option<&'a dyn BlockDevice>,
}

impl<'a> DeviceNamespace<'a> {
    pub const fn new(console: &'static dyn CharacterDevice) -> Self {
        Self {
            console,
            block0: None,
        }
    }

    pub const fn with_block0(
        console: &'static dyn CharacterDevice,
        block0: &'a dyn BlockDevice,
    ) -> Self {
        Self {
            console,
            block0: Some(block0),
        }
    }

    pub fn lookup_path(&self, path: &str) -> Result<VfsNode<'a>, VfsLookupError> {
        if path.is_empty() || !path.starts_with('/') {
            return Err(VfsLookupError::BadPath);
        }

        let mut parts = path.split('/').filter(|part| !part.is_empty());
        match parts.next() {
            None => Ok(VfsNode::directory("/", "/")),
            Some("dev") => self.lookup_dev(parts),
            Some(_) => Err(VfsLookupError::NotFound),
        }
    }

    fn lookup_dev<'p>(
        &self,
        mut parts: impl Iterator<Item = &'p str>,
    ) -> Result<VfsNode<'a>, VfsLookupError> {
        match parts.next() {
            None => Ok(VfsNode::directory("/dev", "dev")),
            Some("console") => {
                if parts.next().is_some() {
                    Err(VfsLookupError::NotDirectory)
                } else {
                    Ok(VfsNode::character("/dev/console", "console", self.console))
                }
            }
            Some("vda") => {
                if parts.next().is_some() {
                    Err(VfsLookupError::NotDirectory)
                } else if let Some(block0) = self.block0 {
                    Ok(VfsNode::block("/dev/vda", "vda", block0))
                } else {
                    Err(VfsLookupError::NotFound)
                }
            }
            Some(_) => Err(VfsLookupError::NotFound),
        }
    }
}
