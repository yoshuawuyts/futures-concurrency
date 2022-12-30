use super::super::shared_slice_waker::{waker_from_position, WakerArrayTrait};
use super::awakeness::AwakenessArray;

use core::array;
use core::task::Waker;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

use bitvec::prelude::BitArray;
use bitvec::slice::BitSlice;

struct BitArrayWrapped<const N: usize>(BitArray<[u8; N]>);

impl<const N: usize> AsMut<BitSlice<u8>> for BitArrayWrapped<N> {
    fn as_mut(&mut self) -> &mut BitSlice<u8> {
        &mut self.0[..N]
    }
}

// Each waker points to a slot in the `wake_data` part of `Inner`.
// Every one of those slots contain a pointer to the Arc wrapping `Inner` itself.
// Wakers figure out their indices by comparing the address they are pointing to to `wake_data`'s start address.
//
//      ┌───► Arc<Inner> ◄─────────┬──────────────┬──────────────┐
//      │                          │              │              │
//      │     ┌─────────────┬──────┼───────┬──────┼───────┬──────┼───────┬─────┐
//      │     │             │      │       │      │       │      │       │     │
//      │     │  Readiness  │ wake_data[0] │ wake_data[1] │ wake_data[2] │ ... │
//      │     │             │              │              │              │     │
//      │     └─────────────┴──────▲───────┴──────▲───────┴──────▲───────┴─────┘
//      │                          │              │              │
//      │         ┌────────────────┘              │              │
//      │         │                               │              │
//      │         │           ┌───────────────────┘              │
//      │         │           │                                  │
//      │         │           │           ┌──────────────────────┘
//      │         │           │           │
//      │         │           │           │
// ┌────┼────┬────┼──────┬────┼──────┬────┼──────┬─────┐
// │    │    │    │      │    │      │    │      │     │
// │  Inner  │ wakers[0] │ wakers[1] │ wakers[2] │ ... │
// │         │           │           │           │     │
// └─────────┴───────────┴───────────┴───────────┴─────┘
// WakerArray/WakerVec
//

/// A collection of wakers which delegate to an in-line waker.
pub(crate) struct WakerArray<const N: usize> {
    inner: Arc<WakerArrayInner<N>>,
    wakers: [Waker; N],
}
struct WakerArrayInner<const N: usize> {
    wake_data: [*const Self; N],
    awakeness: Mutex<AwakenessArray<N>>,
}

impl<const N: usize> WakerArray<N> {
    /// Create a new instance of `WakerArray`.
    pub(crate) fn new() -> Self {
        let mut inner = Arc::new(WakerArrayInner {
            awakeness: Mutex::new(AwakenessArray::new()),
            wake_data: [std::ptr::null(); N], // We don't know the Arc's address yet so put null for now.
        });
        let raw = Arc::into_raw(Arc::clone(&inner)); // The Arc's address.

        // At this point the strong count is 2. Decrement it to 1.
        // Each time we create/clone a Waker the count will be incremented by 1.
        // So N Wakers -> count = N+1.
        unsafe { Arc::decrement_strong_count(raw) }

        // Make wake_data all point to the Arc itself.
        Arc::get_mut(&mut inner).unwrap().wake_data = [raw; N];

        let wakers = array::from_fn(|i| {
            let data = inner.wake_data.get(i).unwrap();
            unsafe {
                waker_from_position::<WakerArrayInner<N>>(data as *const *const WakerArrayInner<N>)
            }
        });
        Self { inner, wakers }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&Waker> {
        self.wakers.get(index)
    }

    pub(crate) fn awakeness(&mut self) -> MutexGuard<'_, AwakenessArray<N>> {
        self.inner.awakeness.lock().unwrap()
    }
}

impl<const N: usize> WakerArrayTrait for WakerArrayInner<N> {
    fn get_wake_data_slice(&self) -> &[*const Self] {
        &self.wake_data
    }

    fn wake_index(&self, index: usize) {
        self.awakeness.lock().unwrap().wake(index);
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::wakers::dummy_waker;

    use super::*;
    #[test]
    fn check_refcount() {
        let mut wa = WakerArray::<5>::new();
        assert_eq!(Arc::strong_count(&wa.inner), 6);
        wa.wakers[4] = dummy_waker();
        assert_eq!(Arc::strong_count(&wa.inner), 5);
        let cloned = wa.wakers[3].clone();
        assert_eq!(Arc::strong_count(&wa.inner), 6);
        wa.wakers[3] = wa.wakers[4].clone();
        assert_eq!(Arc::strong_count(&wa.inner), 5);
        drop(cloned);
        assert_eq!(Arc::strong_count(&wa.inner), 4);

        wa.wakers[0].wake_by_ref();
        wa.wakers[0].wake_by_ref();
        wa.wakers[0].wake_by_ref();
        assert_eq!(Arc::strong_count(&wa.inner), 4);

        wa.wakers[0] = wa.wakers[1].clone();
        assert_eq!(Arc::strong_count(&wa.inner), 4);

        let taken = std::mem::replace(&mut wa.wakers[2], dummy_waker());
        assert_eq!(Arc::strong_count(&wa.inner), 4);
        taken.wake_by_ref();
        taken.wake_by_ref();
        taken.wake_by_ref();
        assert_eq!(Arc::strong_count(&wa.inner), 4);
        taken.wake();
        assert_eq!(Arc::strong_count(&wa.inner), 3);

        wa.wakers = array::from_fn(|_| dummy_waker());
        assert_eq!(Arc::strong_count(&wa.inner), 1);
    }
}
