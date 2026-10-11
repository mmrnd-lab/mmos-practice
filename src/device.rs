// Copyright 2026 Sungjae Park
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Char,
    Block,
}

pub trait Device {
    fn name(&self) -> &str;
    fn kind(&self) -> DeviceKind;
}
