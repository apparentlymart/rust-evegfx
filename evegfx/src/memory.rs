//! Pointers in the EVE memory space.

pub mod region;

mod ptr;
mod slice;

#[doc(inline)]
pub use ptr::Ptr;

#[doc(inline)]
pub use slice::Slice;

pub(crate) use region::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::testing::Exhaustive;
    use crate::models::Model;

    #[test]
    fn test_ptr_eq() {
        assert_eq!(
            <Exhaustive as Model>::MainMem::ptr(1),
            <Exhaustive as Model>::MainMem::ptr(1)
        );
        assert_ne!(
            <Exhaustive as Model>::MainMem::ptr(1),
            <Exhaustive as Model>::MainMem::ptr(2)
        );
        // Addresses from different memory regions on the same model are
        // comparable, but will always return false because the address
        // regions are required to be disjoint within a model.
        assert_ne!(
            <Exhaustive as Model>::MainMem::ptr(1),
            <Exhaustive as Model>::DisplayListMem::ptr(1)
        );
    }

    #[test]
    fn test_ptr_cmp() {
        assert!(<Exhaustive as Model>::MainMem::ptr(1) < <Exhaustive as Model>::MainMem::ptr(2));
        assert!(<Exhaustive as Model>::MainMem::ptr(2) >= <Exhaustive as Model>::MainMem::ptr(2));
        // Addresses from different memory regions on the same model are
        // comparable, and reflect the relative positions of those regions
        // in the memory map. In the case of the "Exhaustive" testing model,
        // MainMem is at lower addresses than DisplayListMem in the memory
        // map.
        assert!(
            <Exhaustive as Model>::MainMem::ptr(1) < <Exhaustive as Model>::DisplayListMem::ptr(1)
        );
        assert!(
            !(<Exhaustive as Model>::MainMem::ptr(1)
                > <Exhaustive as Model>::DisplayListMem::ptr(1))
        );
    }
}
