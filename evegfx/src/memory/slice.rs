use super::ptr::Ptr;
use super::region::MemoryRegion;

/// A consecutive sequence of memory addresses in a particular region.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Slice<R: MemoryRegion> {
    start_: Ptr<R>, // inclusive
    end_: Ptr<R>,   // exclusive
}

impl<R: MemoryRegion> Slice<R> {
    pub fn len(&self) -> u32 {
        self.end_.addr - self.start_.addr
    }

    pub fn contains(&self, ptr: Ptr<R>) -> bool {
        ptr >= self.start_ && ptr < self.end_
    }

    pub fn bounds(self) -> (Ptr<R>, Ptr<R>) {
        (self.start_, self.end_)
    }

    pub fn raw_bounds(self) -> (u32, u32) {
        (self.start_.to_raw(), self.end_.to_raw())
    }
}

impl<R: MemoryRegion> core::convert::From<core::ops::Range<Ptr<R>>> for Slice<R> {
    fn from(range: core::ops::Range<Ptr<R>>) -> Self {
        Self {
            start_: range.start,
            end_: range.end,
        }
    }
}
