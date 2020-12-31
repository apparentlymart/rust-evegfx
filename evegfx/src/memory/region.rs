//! Types for representing EVE memory regions at compile time.

use super::ptr::Ptr;
use crate::models::Model;
use core::marker::PhantomData;

/// A trait implemented by all memory regions that [`Ptr`](Ptr) instances can
/// refer to.
///
/// It doesn't make sense to implement this trait outside of the `evegfx`
/// crate. It is implemented by EVE-model-specific APIs elsewhere in this
/// crate. Within the context of a particular model none of the available
/// memory regions may overlap.
///
/// Memory regions exist only at compile time, as a facility to have the
/// Rust type system help ensure valid use of pointers. At runtime we
/// deal only in absolute addresses represented as u32.
pub trait MemoryRegion: core::marker::Sized + core::fmt::Debug + core::marker::Copy {
    type Model: Model;

    const BASE_ADDR: u32;
    const LENGTH: u32;
    const DEBUG_NAME: &'static str;

    /// Creates a pointer in the selected memory region.
    ///
    /// The given value is interpreted as an offset into the memory region,
    /// modulo the size of the region.
    #[inline]
    fn ptr(raw: u32) -> Ptr<Self> {
        Ptr {
            addr: Self::BASE_ADDR + (raw % Self::LENGTH),
            _region: PhantomData,
        }
    }
}

pub trait MainMem: MemoryRegion + HostAccessible {}

pub trait FontMem: MemoryRegion + HostAccessible {}

pub trait DisplayListMem: MemoryRegion + HostAccessible {}

pub trait RegisterMem: MemoryRegion + HostAccessible {}

pub trait CommandMem: MemoryRegion + HostAccessible {}

pub trait CommandErrMem: MemoryRegion + HostAccessible {
    type RawMessage: crate::commands::coprocessor::FaultMessageRaw;
}

/// Implemented by memory regions that can be accessed indirectly via the
/// `CMD_FLASH...` family of coprocessor commands.
pub trait ExtFlashMem: MemoryRegion {}

/// Implemented by memory regions that can be directly read or written by
/// the host controller. Memory regions implementing this trait may only
/// use the lower 22 bits of the address space, with the topmost 10 bits
/// always set to zero.
pub trait HostAccessible: MemoryRegion {}
