pub mod bt815;

use crate::memory;

/// Implemented by types that represent the characteristics of different
/// specific models of EVE.
///
/// Although the Rust compiler would allow implementations of this elsewhere,
/// this trait is intended only for implementation inside this crate and its
/// requirements are subject to change in future, even in minor releases.
///
/// This type is typically implemented on empty struct types so that the
/// name can be used both as a type and as a value (the only value of the
/// type). Models are relevant only at compile time to help generate the
/// correct address offsets for the different memory maps in different
/// generations of the EVE line. They should be removed altogether by the
/// compiler so as to have no appreciable effect at runtime.
pub trait Model: Sized + core::fmt::Debug {
    type MainMem: memory::MainMem;
    type DisplayListMem: memory::DisplayListMem;
    type RegisterMem: memory::RegisterMem;
    type CommandMem: memory::CommandMem;

    fn new_low_level<I: crate::Interface>(ei: I) -> crate::low_level::LowLevel<Self, I> {
        crate::low_level::LowLevel::new(ei)
    }

    fn new<I: crate::Interface>(ei: I) -> crate::EVE<Self, I> {
        crate::EVE::new_internal(ei)
    }

    fn reg_ptr(reg: crate::registers::EVERegister) -> crate::memory::Ptr<Self::RegisterMem> {
        reg.ptr::<Self>()
    }
}

/// Implemented by model types that have an external flash memory space.
pub trait WithExtFlashMem: Model {
    type ExtFlashMem: memory::ExtFlashMem;
}

/// Implemented by model types that have some memory space set aside for
/// reporting coprocessor fault messages.
pub trait WithCommandErrMem: Model {
    type CommandErrMem: memory::CommandErrMem;
}

pub(crate) mod testing {
    use super::*;
    use crate::memory;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

    impl WithCommandErrMem for Exhaustive {
        type CommandErrMem = CommandErrMem;
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub(crate) enum MainMem {}
    impl memory::MemoryRegion for MainMem {
        type Model = Exhaustive;
        const BASE_ADDR: u32 = 0x000000;
        const LENGTH: u32 = 1024 * 1024;
        const DEBUG_NAME: &'static str = "MainMem";
    }
    impl memory::HostAccessible for MainMem {}
    impl memory::MainMem for MainMem {}

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub(crate) enum DisplayListMem {}
    impl memory::MemoryRegion for DisplayListMem {
        type Model = Exhaustive;
        const BASE_ADDR: u32 = 0x300000;
        const LENGTH: u32 = 8 * 1024;
        const DEBUG_NAME: &'static str = "DisplayListMem";
    }
    impl memory::HostAccessible for DisplayListMem {}
    impl memory::DisplayListMem for DisplayListMem {}

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub(crate) enum RegisterMem {}
    impl memory::MemoryRegion for RegisterMem {
        type Model = Exhaustive;
        const BASE_ADDR: u32 = 0x302000;
        const LENGTH: u32 = 4 * 1024;
        const DEBUG_NAME: &'static str = "RegisterMem";
    }
    impl memory::HostAccessible for RegisterMem {}
    impl memory::RegisterMem for RegisterMem {}

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub(crate) enum CommandMem {}
    impl memory::MemoryRegion for CommandMem {
        type Model = Exhaustive;
        const BASE_ADDR: u32 = 0x308000;
        const LENGTH: u32 = 4 * 1024;
        const DEBUG_NAME: &'static str = "CommandMem";
    }
    impl memory::HostAccessible for CommandMem {}
    impl memory::CommandMem for CommandMem {}

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub(crate) enum CommandErrMem {}
    impl memory::MemoryRegion for CommandErrMem {
        type Model = Exhaustive;
        const BASE_ADDR: u32 = 0x309800;
        const LENGTH: u32 = 128;
        const DEBUG_NAME: &'static str = "CommandErrMem";
    }
    impl memory::HostAccessible for CommandErrMem {}
    impl memory::CommandErrMem for CommandErrMem {
        type RawMessage = [u8; 128];
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub(crate) enum ExtFlashMem {}
    impl memory::MemoryRegion for ExtFlashMem {
        type Model = Exhaustive;
        const BASE_ADDR: u32 = 0x800000;
        const LENGTH: u32 = 256 * 1024 * 1024;
        const DEBUG_NAME: &'static str = "ExtFlashMem";
    }
    impl memory::ExtFlashMem for ExtFlashMem {}
}
