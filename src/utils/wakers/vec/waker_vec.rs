use std::sync::Arc;
use std::sync::Mutex;
use std::task::Waker;

use crate::utils::wakers::shared_slice_waker::waker_from_position;
use crate::utils::wakers::shared_slice_waker::WakerArrayTrait;

use super::ReadinessVec;

/// A collection of wakers which delegate to an in-line waker.
pub(crate) struct WakerVec {
    inner: Arc<WakerVecInner>,
    wakers: Vec<Waker>,
}

struct WakerVecInner {
    wake_data: Vec<*const Self>,
    readiness: Mutex<ReadinessVec>,
}

impl WakerVec {
    /// Create a new instance of `WakerVec`.
    pub(crate) fn new(len: usize) -> Self {
        let readiness = Mutex::new(ReadinessVec::new(len));
        let mut inner = Arc::new(WakerVecInner {
            readiness,
            wake_data: vec![std::ptr::null(); len],
        });
        let raw = Arc::into_raw(Arc::clone(&inner));
        unsafe { Arc::decrement_strong_count(raw) }
        Arc::get_mut(&mut inner)
            .unwrap()
            .wake_data
            .iter_mut()
            .for_each(|item| *item = raw);
        let wakers = inner
            .wake_data
            .iter()
            .map(|data| unsafe {
                waker_from_position::<WakerVecInner>(data as *const *const WakerVecInner)
            })
            .collect();
        Self { inner, wakers }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&Waker> {
        self.wakers.get(index)
    }

    /// Access the `Readiness`.
    pub(crate) fn readiness(&self) -> &Mutex<ReadinessVec> {
        &self.inner.readiness
    }
}

impl WakerArrayTrait for WakerVecInner {
    fn get_wake_data_slice(&self) -> &[*const Self] {
        &self.wake_data
    }

    fn wake_index(&self, index: usize) {
        self.readiness.lock().unwrap().wake(index)
    }
}
