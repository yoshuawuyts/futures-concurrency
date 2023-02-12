use core::task::Waker;
use std::sync::{Arc, Mutex};

use super::{
    super::shared_arc::{waker_from_redirect_position, SharedArcContent},
    ReadinessVec,
};

/// A collection of wakers sharing the same allocation.
pub(crate) struct WakerVec {
    wakers: Vec<Waker>,
    inner: Arc<WakerVecInner>,
}

/// See [super::super::shared_arc] for how this works.
struct WakerVecInner {
    redirect: Vec<*const Self>,
    readiness: Mutex<ReadinessVec>,
}

impl WakerVec {
    /// Create a new instance of `WakerVec`.
    pub(crate) fn new(len: usize) -> Self {
        let mut inner = Arc::new(WakerVecInner {
            readiness: Mutex::new(ReadinessVec::new(len)),
            redirect: Vec::new(),
        });
        let raw = Arc::into_raw(Arc::clone(&inner)); // The Arc's address.

        // At this point the strong count is 2. Decrement it to 1.
        // Each time we create/clone a Waker the count will be incremented by 1.
        // So N Wakers -> count = N+1.
        unsafe { Arc::decrement_strong_count(raw) }

        // Make redirect all point to the Arc itself.
        Arc::get_mut(&mut inner).unwrap().redirect = vec![raw; len];

        // Now the redirect vec is complete. Time to create the actual Wakers.
        let wakers = inner
            .redirect
            .iter()
            .map(|data| unsafe {
                waker_from_redirect_position::<WakerVecInner>(data as *const *const WakerVecInner)
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

impl SharedArcContent for WakerVecInner {
    fn get_redirect_slice(&self) -> &[*const Self] {
        &self.redirect
    }

    fn wake_index(&self, index: usize) {
        let mut readiness = self.readiness.lock().unwrap();
        if !readiness.set_ready(index) {
            readiness
                .parent_waker()
                .as_ref()
                .expect("`parent_waker` not available from `Readiness`. Did you forget to call `Readiness::set_waker`?")
                .wake_by_ref();
        }
    }
}
