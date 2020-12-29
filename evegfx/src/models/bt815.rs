use super::{Model, WithExtFlashMem};
use crate::memory;

/// Device type representing the BT815 and BT816 models.
///
/// This type is used only at compile time as a type parameter, or as an
/// empty (compile-time-only) argument in order to influence selection of
/// a type parameter on a function call that wouldn't naturally imply one.
///
/// To use the main [`EVE`](crate::EVE) API with this model, pass the model
/// to [`EVE::new`](crate::EVE::new) along with a suitable
/// [`Interface`](crate::Interface) for your underlying platform.

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BT815;

impl Model for BT815 {
    type MainMem = MainMem;
    type DisplayListMem = DisplayListMem;
    type RegisterMem = RegisterMem;
    type CommandMem = CommandMem;
}

impl WithExtFlashMem for BT815 {
    type ExtFlashMem = ExtFlashMem;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MainMem {}
impl memory::MemoryRegion for MainMem {
    type Model = BT815;
    const BASE_ADDR: u32 = 0x000000;
    const LENGTH: u32 = 1024 * 1024;
    const DEBUG_NAME: &'static str = "MainMem";
}
impl memory::HostAccessible for MainMem {}
impl memory::MainMem for MainMem {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DisplayListMem {}
impl memory::MemoryRegion for DisplayListMem {
    type Model = BT815;
    const BASE_ADDR: u32 = 0x300000;
    const LENGTH: u32 = 8 * 1024;
    const DEBUG_NAME: &'static str = "DisplayListMem";
}
impl memory::HostAccessible for DisplayListMem {}
impl memory::DisplayListMem for DisplayListMem {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RegisterMem {}
impl memory::MemoryRegion for RegisterMem {
    type Model = BT815;
    const BASE_ADDR: u32 = 0x302000;
    const LENGTH: u32 = 4 * 1024;
    const DEBUG_NAME: &'static str = "RegisterMem";
}
impl memory::HostAccessible for RegisterMem {}
impl memory::RegisterMem for RegisterMem {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CommandMem {}
impl memory::MemoryRegion for CommandMem {
    type Model = BT815;
    const BASE_ADDR: u32 = 0x308000;
    const LENGTH: u32 = 4 * 1024;
    const DEBUG_NAME: &'static str = "CommandMem";
}
impl memory::HostAccessible for CommandMem {}
impl memory::CommandMem for CommandMem {}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ExtFlashMem {}
impl memory::MemoryRegion for ExtFlashMem {
    type Model = BT815;
    const BASE_ADDR: u32 = 0x800000;
    const LENGTH: u32 = 256 * 1024 * 1024;
    const DEBUG_NAME: &'static str = "ExtFlashMem";
}
impl memory::ExtFlashMem for ExtFlashMem {}
