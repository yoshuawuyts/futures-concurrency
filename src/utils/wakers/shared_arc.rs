//! To save on allocations, we avoid making a separate Arc Waker for every subfuture.
//! Rather, we have all N Wakers share a single Arc, and use a "redirect" mechanism to allow different wakers to be distinguished.
//! The mechanism works as follows.
//! The Arc contains 2 things:
//! - the Readiness structure ([ReadinessArray][super::array::ReadinessArray] / [ReadinessVec][super::vec::ReadinessVec])
//! - the redirect array.
//! The redirect array contains N repeated copies of the pointer to the Arc itself (obtained by `Arc::into_raw`).
//! The Waker for the `i`th subfuture points to the `i`th item in the redirect array.
//! (i.e. the Waker pointer is `*const *const A` where `A` is the type of the item in the Arc)
//! When the Waker is woken, we deref it twice (giving reference to the content of the Arc),
//! and compare it to the address of the redirect slice.
//! The difference tells us the index of the waker. We can then record this woken index in the Readiness.
//!
//! ```text
//!    ┌───────────────────────────┬──────────────┬──────────────┐
//!    │                           │              │              │
//!    │    / ┌─────────────┬──────┼───────┬──────┼───────┬──────┼───────┬─────┐ \
//!    ▼   /  │             │      │       │      │       │      │       │     │  \
//!   Arc <   │  Readiness  │  redirect[0] │  redirect[1] │  redirect[2] │ ... │   >
//!    ▲   \  │             │              │              │              │     │  /
//!    │    \ └─────────────┴──────▲───────┴──────▲───────┴──────▲───────┴─────┘ /
//!    │                           │              │              │
//!    └─┐         ┌───────────────┘              │              │
//!      │         │                              │              │
//!      │         │           ┌──────────────────┘              │
//!      │         │           │                                 │
//!      │         │           │           ┌─────────────────────┘
//!      │         │           │           │
//!      │         │           │           │
//! ┌────┼────┬────┼──────┬────┼──────┬────┼──────┬─────┐
//! │    │    │    │      │    │      │    │      │     │
//! │         │ wakers[0] │ wakers[1] │ wakers[2] │ ... │
//! │         │           │           │           │     │
//! └─────────┴───────────┴───────────┴───────────┴─────┘
//! ```

// TODO: Right now each waker gets its own redirect slot.
// We can save space by making size_of::<*const _>() wakers share the same slot.
// With such change, in 64-bit system, the redirect array/vec would only need ⌈N/8⌉ slots instead of N.

use core::task::{RawWaker, RawWakerVTable, Waker};
use std::sync::Arc;

/// A trait to be implemented on [super::WakerArray] and [super::WakerVec] for polymorphism.
/// These are the type that goes in the Arc. They both contain the Readiness and the redirect array/vec.
pub(super) trait SharedArcContent {
    /// Get the reference of the redirect slice. This is used to compute the index.
    fn get_redirect_slice(&self) -> &[*const Self];
    /// Called when the subfuture at the specified index should be polled.
    /// Should call `Readiness::set_ready`.
    fn wake_index(&self, index: usize);
}

/// Create one waker following the mechanism described in the [module][self] doc.
/// The following must be upheld for safety:
/// - `pointer` must points to a slot in the redirect array.
/// - that slot must contain a pointer obtained by `Arc::<A>::into_raw`.
/// - the Arc must still be alive at the time this function is called.
/// The following should be upheld for correct behavior:
/// - calling `SharedArcContent::get_redirect_slice` on the content of the Arc should give the redirect array within which `pointer` points to.
#[deny(unsafe_op_in_unsafe_fn)]
pub(super) unsafe fn waker_from_redirect_position<A: SharedArcContent>(
    pointer: *const *const A,
) -> Waker {
    /// Create a Waker from a type-erased pointer.
    /// The pointer must satisfy the safety constraints listed in the wrapping function's documentation.
    unsafe fn create_waker<A: SharedArcContent>(pointer: *const ()) -> RawWaker {
        // Retype the type-erased pointer.
        let pointer = pointer as *const *const A;

        // We're creating a new Waker, so we need to increment the count.
        // SAFETY: The constraints listed for the wrapping function documentation means
        // - `*pointer` is an `*const A` obtained from `Arc::<A>::into_raw`.
        // - the Arc is alive.
        // So this operation is safe.
        unsafe { Arc::increment_strong_count(*pointer) };

        RawWaker::new(pointer as *const (), create_vtable::<A>())
    }

    /// Invoke `SharedArcContent::wake_index` with the index in the redirect slice where this pointer points to.
    /// The pointer must satisfy the safety constraints listed in the wrapping function's documentation.
    unsafe fn wake_by_ref<A: SharedArcContent>(pointer: *const ()) {
        // Retype the type-erased pointer.
        let pointer = pointer as *const *const A;

        // SAFETY: we are already requiring `pointer` to point to a slot in the redirect array.
        let raw: *const A = unsafe { *pointer };
        // SAFETY: we are already requiring the pointer in the redirect array slot to be obtained from `Arc::into_raw`.
        let arc_content: &A = unsafe { &*raw };

        // Calculate the index.
        // This is your familiar pointer math
        // `item_address = array_address + (index * item_size)`
        // rearranged to
        // `index = (item_address - array_address) / item_size`.
        let item_address = sptr::Strict::addr(pointer);
        let redirect_slice_address = sptr::Strict::addr(arc_content.get_redirect_slice().as_ptr());
        let redirect_item_size = core::mem::size_of::<*const A>(); // the size of each item in the redirect slice
        let index = (item_address - redirect_slice_address) / redirect_item_size;

        arc_content.wake_index(index);
    }

    /// The pointer must satisfy the safety constraints listed in the wrapping function's documentation.
    unsafe fn drop_waker<A: SharedArcContent>(pointer: *const ()) {
        // Retype the type-erased pointer.
        let pointer = pointer as *const *const A;

        // SAFETY: we are already requiring `pointer` to point to a slot in the redirect array.
        let raw = unsafe { *pointer };
        // SAFETY: we are already requiring the pointer in the redirect array slot to be obtained from `Arc::into_raw`.
        unsafe { Arc::decrement_strong_count(raw) };
    }

    /// The pointer must satisfy the safety constraints listed in the wrapping function's documentation.
    unsafe fn wake<A: SharedArcContent>(pointer: *const ()) {
        // SAFETY: we are already requiring the constraints of `wake_by_ref` and `drop_waker`.
        unsafe {
            wake_by_ref::<A>(pointer);
            drop_waker::<A>(pointer);
        }
    }

    fn create_vtable<A: SharedArcContent>() -> &'static RawWakerVTable {
        &RawWakerVTable::new(
            create_waker::<A>,
            wake::<A>,
            wake_by_ref::<A>,
            drop_waker::<A>,
        )
    }
    // SAFETY: All our vtable functions adhere to the RawWakerVTable contract,
    // and we are already requiring that `pointer` is what our functions expect.
    unsafe { Waker::from_raw(create_waker::<A>(pointer as *const ())) }
}
