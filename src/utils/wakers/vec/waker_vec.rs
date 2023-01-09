use core::task::Waker;
use std::sync::{Arc, Mutex};

use super::{
    super::shared_arc::{waker_for_wake_data_slot, WakeDataContainer},
    ReadinessVec,
};

/// A collection of wakers sharing the same allocation.
pub(crate) struct WakerVec {
    wakers: Vec<Waker>,
    inner: Arc<WakerVecInner>,
}

/// See [super::super::shared_arc] for how this works.
struct WakerVecInner {
    wake_data: Vec<*const Self>,
    readiness: Mutex<ReadinessVec>,
}

impl WakerVec {
    /// Create a new instance of `WakerVec`.
    pub(crate) fn new(len: usize) -> Self {
        let mut inner = Arc::new(WakerVecInner {
            readiness: Mutex::new(ReadinessVec::new(len)),
            wake_data: Vec::new(),
        });
        let raw = Arc::into_raw(Arc::clone(&inner)); // The Arc's address.

        // At this point the strong count is 2. Decrement it to 1.
        // Each time we create/clone a Waker the count will be incremented by 1.
        // So N Wakers -> count = N+1.
        unsafe { Arc::decrement_strong_count(raw) }

        // Make wake_data all point to the Arc itself.
        Arc::get_mut(&mut inner).unwrap().wake_data = vec![raw; len];

        // Now the wake_data vec is complete. Time to create the actual Wakers.
        let wakers = inner
            .wake_data
            .iter()
            .map(|data| unsafe {
                waker_for_wake_data_slot::<WakerVecInner>(data as *const *const WakerVecInner)
            })
            .collect();

        Self { inner, wakers }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&Waker> {
        self.wakers.get(index)
    }

    pub(crate) fn readiness(&self) -> &Mutex<ReadinessVec> {
        &self.inner.readiness
    }
}

impl WakeDataContainer for WakerVecInner {
    fn get_wake_data_slice(&self) -> &[*const Self] {
        &self.wake_data
    }

    fn wake_index(&self, index: usize) {
        self.readiness.lock().unwrap().wake(index);
    }
}
