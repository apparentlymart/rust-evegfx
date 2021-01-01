//! A fake implementation of `Model` for testing and examples.

use crate::memory;

/// A fake implementation of `Model` for testing and examples.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Model;

impl super::Model for Model {
    type MainMem = MainMem;
    type DisplayListMem = DisplayListMem;
    type RegisterMem = RegisterMem;
    type CommandMem = CommandMem;
}

impl super::WithExtFlashMem for Model {
    type ExtFlashMem = ExtFlashMem;
}

impl super::WithCommandErrMem for Model {
    type CommandErrMem = CommandErrMem;
}

impl super::WithCoprocessorAPILevel1 for Model {}
impl super::WithCoprocessorAPILevel2 for Model {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MainMem {}
impl memory::MemoryRegion for MainMem {
    type Model = Model;
    const BASE_ADDR: u32 = 0x000000;
    const LENGTH: u32 = 1024 * 1024;
    const DEBUG_NAME: &'static str = "MainMem";
}
impl memory::HostAccessible for MainMem {}
impl memory::MainMem for MainMem {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DisplayListMem {}
impl memory::MemoryRegion for DisplayListMem {
    type Model = Model;
    const BASE_ADDR: u32 = 0x300000;
    const LENGTH: u32 = 8 * 1024;
    const DEBUG_NAME: &'static str = "DisplayListMem";
}
impl memory::HostAccessible for DisplayListMem {}
impl memory::DisplayListMem for DisplayListMem {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RegisterMem {}
impl memory::MemoryRegion for RegisterMem {
    type Model = Model;
    const BASE_ADDR: u32 = 0x302000;
    const LENGTH: u32 = 4 * 1024;
    const DEBUG_NAME: &'static str = "RegisterMem";
}
impl memory::HostAccessible for RegisterMem {}
impl memory::RegisterMem for RegisterMem {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CommandMem {}
impl memory::MemoryRegion for CommandMem {
    type Model = Model;
    const BASE_ADDR: u32 = 0x308000;
    const LENGTH: u32 = 4 * 1024;
    const DEBUG_NAME: &'static str = "CommandMem";
}
impl memory::HostAccessible for CommandMem {}
impl memory::CommandMem for CommandMem {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CommandErrMem {}
impl memory::MemoryRegion for CommandErrMem {
    type Model = Model;
    const BASE_ADDR: u32 = 0x309800;
    const LENGTH: u32 = 128;
    const DEBUG_NAME: &'static str = "CommandErrMem";
}
impl memory::HostAccessible for CommandErrMem {}
impl memory::CommandErrMem for CommandErrMem {
    type RawMessage = [u8; 128];
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ExtFlashMem {}
impl memory::MemoryRegion for ExtFlashMem {
    type Model = Model;
    const BASE_ADDR: u32 = 0x800000;
    const LENGTH: u32 = 256 * 1024 * 1024;
    const DEBUG_NAME: &'static str = "ExtFlashMem";
}
impl memory::ExtFlashMem for ExtFlashMem {}
