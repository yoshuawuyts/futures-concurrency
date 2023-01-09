use core::array;
use core::task::Waker;
use std::sync::{Arc, Mutex};

use super::{
    super::shared_arc::{waker_for_wake_data_slot, WakeDataContainer},
    ReadinessArray,
};

/// A collection of wakers which delegate to an in-line waker.
pub(crate) struct WakerArray<const N: usize> {
    wakers: [Waker; N],
    inner: Arc<WakerArrayInner<N>>,
}

/// See [super::super::shared_arc] for how this works.
struct WakerArrayInner<const N: usize> {
    wake_data: [*const Self; N],
    readiness: Mutex<ReadinessArray<N>>,
}

impl<const N: usize> WakerArray<N> {
    /// Create a new instance of `WakerArray`.
    pub(crate) fn new() -> Self {
        let mut inner = Arc::new(WakerArrayInner {
            readiness: Mutex::new(ReadinessArray::new()),
            wake_data: [std::ptr::null(); N], // We don't know the Arc's address yet so put null for now.
        });
        let raw = Arc::into_raw(Arc::clone(&inner)); // The Arc's address.

        // At this point the strong count is 2. Decrement it to 1.
        // Each time we create/clone a Waker the count will be incremented by 1.
        // So N Wakers -> count = N+1.
        unsafe { Arc::decrement_strong_count(raw) }

        // Make wake_data all point to the Arc itself.
        Arc::get_mut(&mut inner).unwrap().wake_data = [raw; N];

        // Now the wake_data array is complete. Time to create the actual Wakers.
        let wakers = array::from_fn(|i| {
            let data = inner.wake_data.get(i).unwrap();
            unsafe {
                waker_for_wake_data_slot::<WakerArrayInner<N>>(
                    data as *const *const WakerArrayInner<N>,
                )
            }
        });

        Self { inner, wakers }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&Waker> {
        self.wakers.get(index)
    }

    /// Access the `Readiness`.
    pub(crate) fn readiness(&self) -> &Mutex<ReadinessArray<N>> {
        &self.inner.readiness
    }
}

impl<const N: usize> WakeDataContainer for WakerArrayInner<N> {
    fn get_wake_data_slice(&self) -> &[*const Self] {
        &self.wake_data
    }

    fn wake_index(&self, index: usize) {
        self.readiness.lock().unwrap().wake(index);
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
