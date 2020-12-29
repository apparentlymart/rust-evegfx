pub mod bt815;

use crate::memory;

/// Implemented by types that represent the characteristics of different
/// specific models of EVE.
///
/// Although the Rust compiler would allow implementations of this elsewhere,
/// this trait is intended only for implementation inside this crate and its
/// requirements are subject to change in future, even in minor releases.
///
/// This type is typically implemented on empty enum types to represent that
/// models are a compile-time-only construct used to represent the minor
/// differences between models through monomorphization, and they have no
/// presence at runtime.
pub trait Model: Sized {
    type MainMem: memory::MainMem;
    type DisplayListMem: memory::DisplayListMem;
    type RegisterMem: memory::RegisterMem;
    type CommandMem: memory::CommandMem;

    fn new_low_level<I: crate::Interface>(ei: I) -> crate::low_level::LowLevel<Self, I> {
        crate::low_level::LowLevel::new(ei)
    }

    fn new<I: crate::Interface>(ei: I) -> crate::EVE<Self, I> {
        crate::EVE::new(ei)
    }

    fn reg_ptr(reg: crate::registers::EVERegister) -> crate::memory::Ptr<Self::RegisterMem> {
        reg.ptr::<Self>()
    }
}

/// Implemented by model types that have an external flash memory space.
pub trait WithExtFlashMem: Model {
    type ExtFlashMem: memory::ExtFlashMem;
}

pub(crate) mod testing {
    use super::*;
    use crate::memory;

    pub(crate) enum Exhaustive {}

    impl Model for Exhaustive {
        type MainMem = MainMem;
        type DisplayListMem = DisplayListMem;
        type RegisterMem = RegisterMem;
        type CommandMem = CommandMem;
    }

    impl WithExtFlashMem for Exhaustive {
        type ExtFlashMem = ExtFlashMem;
    }

    pub(crate) enum MainMem {}
    impl memory::MemoryRegion for MainMem {
        const BASE_ADDR: u32 = 0x000000;
        const LENGTH: u32 = 1024 * 1024;
        const DEBUG_NAME: &'static str = "MainMem";
    }
    impl memory::HostAccessible for MainMem {}
    impl memory::MainMem for MainMem {}

    pub(crate) enum DisplayListMem {}
    impl memory::MemoryRegion for DisplayListMem {
        const BASE_ADDR: u32 = 0x300000;
        const LENGTH: u32 = 8 * 1024;
        const DEBUG_NAME: &'static str = "DisplayListMem";
    }
    impl memory::HostAccessible for DisplayListMem {}
    impl memory::DisplayListMem for DisplayListMem {}

    pub(crate) enum RegisterMem {}
    impl memory::MemoryRegion for RegisterMem {
        const BASE_ADDR: u32 = 0x302000;
        const LENGTH: u32 = 4 * 1024;
        const DEBUG_NAME: &'static str = "RegisterMem";
    }
    impl memory::HostAccessible for RegisterMem {}
    impl memory::RegisterMem for RegisterMem {}

    pub(crate) enum CommandMem {}
    impl memory::MemoryRegion for CommandMem {
        const BASE_ADDR: u32 = 0x308000;
        const LENGTH: u32 = 4 * 1024;
        const DEBUG_NAME: &'static str = "CommandMem";
    }
    impl memory::HostAccessible for CommandMem {}
    impl memory::CommandMem for CommandMem {}

    pub(crate) enum ExtFlashMem {}
    impl memory::MemoryRegion for ExtFlashMem {
        const BASE_ADDR: u32 = 0x800000;
        const LENGTH: u32 = 256 * 1024 * 1024;
        const DEBUG_NAME: &'static str = "ExtFlashMem";
    }
    impl memory::ExtFlashMem for ExtFlashMem {}
}
