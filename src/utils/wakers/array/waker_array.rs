use alloc::sync::{Arc, Weak};
use core::array;
use core::task::Waker;
use std::sync::{Mutex, MutexGuard};

use crate::utils::wakers::shared_arc::{waker_from_redirec_position, SharedArcContent};

use super::{InlineWakerArray, ReadinessArray};

/// A collection of wakers which delegate to an in-line waker.
pub(crate) struct WakerArray<const N: usize> {
    wakers: [Waker; N],
    inner: Arc<WakerArrayInner<N>>,
}

impl<const N: usize> WakerArray<N> {
    /// Create a new instance of `WakerArray`.
    pub(crate) fn new() -> Self {
        let inner = Arc::new_cyclic(|w| {
            let raw = Weak::as_ptr(w);
            WakerArrayInner {
                redirect: [raw; N],
                readiness: Mutex::new(ReadinessArray::new()),
            }
        });
        let wakers =
            array::from_fn(|i| unsafe { waker_from_redirec_position(Arc::clone(&inner), i) });
        Self { wakers, inner }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&Waker> {
        self.wakers.get(index)
    }

    /// Access the `Readiness`.
    pub(crate) fn readiness(&mut self) -> MutexGuard<'_, ReadinessArray<N>> {
        self.inner.readiness.lock().unwrap() // TODO: unwrap
    }
}

struct WakerArrayInner<const N: usize> {
    redirect: [*const Self; N],
    readiness: Mutex<ReadinessArray<N>>,
}

unsafe impl<const N: usize> SharedArcContent for WakerArrayInner<N> {
    fn get_redirect_slice(&self) -> &[*const Self] {
        &self.redirect
    }

    fn wake_index(&self, index: usize) {
        let mut readiness = self.readiness.lock().unwrap(); // TODO: unwrap
        if !readiness.set_ready(index) {
            readiness
                .parent_waker()
                .as_ref()
                .expect("`parent_waker` not available from `Readiness`. Did you forget to call `Readiness::set_waker`?")
                .wake_by_ref()
        }
    }
}
