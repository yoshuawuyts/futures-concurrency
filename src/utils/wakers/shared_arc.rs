use core::task::{RawWaker, RawWakerVTable, Waker};
use std::sync::Arc;

// In the diagram below, `A` is the upper block.
// It is a struct that implements WakeDataContainer (so either WakerVecInner or WakerArrayInner).
// The lower block is either WakerVec or WakerArray. Each waker there points to a slot of wake_data in `A`.
// Every one of these slots contain a pointer to the Arc wrapping `A` itself.
// Wakers figure out their indices by comparing the address they are pointing to to `wake_data`'s start address.
//
//    ┌───────────────────────────┬──────────────┬──────────────┐
//    │                           │              │              │
//    │    / ┌─────────────┬──────┼───────┬──────┼───────┬──────┼───────┬─────┐ \
//    ▼   /  │             │      │       │      │       │      │       │     │  \
//   Arc <   │  Readiness  │ wake_data[0] │ wake_data[1] │ wake_data[2] │ ... │   >
//    ▲   \  │             │              │              │              │     │  /
//    │    \ └─────────────┴──────▲───────┴──────▲───────┴──────▲───────┴─────┘ /
//    │                           │              │              │
//    └─┐         ┌───────────────┘              │              │
//      │         │                              │              │
//      │         │           ┌──────────────────┘              │
//      │         │           │                                 │
//      │         │           │           ┌─────────────────────┘
//      │         │           │           │
//      │         │           │           │
// ┌────┼────┬────┼──────┬────┼──────┬────┼──────┬─────┐
// │    │    │    │      │    │      │    │      │     │
// │  Inner  │ wakers[0] │ wakers[1] │ wakers[2] │ ... │
// │         │           │           │           │     │
// └─────────┴───────────┴───────────┴───────────┴─────┘

// TODO: Right now each waker gets its own wake_data slot.
// We can save space by making size_of::<usize>() wakers share the same slot.
// With such change, in 64-bit system, the wake_data array/vec would only need ⌈N/8⌉ slots instead of N.

pub(super) trait WakeDataContainer {
    /// Get the reference of the wake_data slice. This is used to compute the index.
    fn get_wake_data_slice(&self) -> &[*const Self];
    /// Called when the subfuture at the specified index should be polled.
    fn wake_index(&self, index: usize);
}
pub(super) unsafe fn waker_for_wake_data_slot<A: WakeDataContainer>(
    pointer: *const *const A,
) -> Waker {
    unsafe fn clone_waker<A: WakeDataContainer>(pointer: *const ()) -> RawWaker {
        let pointer = pointer as *const *const A;
        let raw = *pointer; // This is the raw pointer of Arc<Inner>.

        // We're creating a new Waker, so we need to increment the count.
        Arc::increment_strong_count(raw);

        RawWaker::new(pointer as *const (), create_vtable::<A>())
    }

    // Convert a pointer to a wake_data slot to the Arc<Inner>.
    unsafe fn to_arc<A: WakeDataContainer>(pointer: *const *const A) -> Arc<A> {
        let raw = *pointer;
        Arc::from_raw(raw)
    }
    unsafe fn wake<A: WakeDataContainer, const BY_REF: bool>(pointer: *const ()) {
        let pointer = pointer as *const *const A;
        let arc = to_arc::<A>(pointer);
        // Calculate the index
        let index = ((pointer as usize) // This is the slot our pointer points to.
            - (arc.get_wake_data_slice() as *const [*const A] as *const () as usize)) // This is the starting address of wake_data.
            / std::mem::size_of::<*const A>();

        arc.wake_index(index);

        // Dropping the Arc would decrement the strong count.
        // We only want to do that when we're not waking by ref.
        if BY_REF {
            std::mem::forget(arc);
        } else {
            std::mem::drop(arc);
        }
    }
    unsafe fn drop_waker<A: WakeDataContainer>(pointer: *const ()) {
        let pointer = pointer as *const *const A;
        let arc = to_arc::<A>(pointer);
        // Decrement the strong count by dropping the Arc.
        std::mem::drop(arc);
    }
    fn create_vtable<A: WakeDataContainer>() -> &'static RawWakerVTable {
        &RawWakerVTable::new(
            clone_waker::<A>,
            wake::<A, false>,
            wake::<A, true>,
            drop_waker::<A>,
        )
    }
    Waker::from_raw(clone_waker::<A>(pointer as *const ()))
}
