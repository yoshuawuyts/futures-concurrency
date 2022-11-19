use std::sync::Arc;
use std::sync::Mutex;
use std::task::Waker;

use super::{InlineWakerVec, ReadinessVec};

/// A collection of wakers which delegate to an in-line waker.
pub(crate) struct WakerVec {
    wakers: Vec<Waker>,
    readiness: Arc<Mutex<ReadinessVec>>,
}

impl WakerVec {
    /// Create a new instance of `WakerVec`.
    pub(crate) fn new(len: usize) -> Self {
        let readiness = Arc::new(Mutex::new(ReadinessVec::new(len)));
        let wakers = (0..len)
            .map(|i| Arc::new(InlineWakerVec::new(i, readiness.clone())).into())
            .collect();
        Self { wakers, readiness }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&Waker> {
        self.wakers.get(index)
    }

    /// Access the `Readiness`.
    pub(crate) fn readiness(&self) -> &Mutex<ReadinessVec> {
        self.readiness.as_ref()
    }
}
