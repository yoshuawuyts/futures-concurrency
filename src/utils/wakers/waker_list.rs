use std::sync::Arc;
use std::sync::Mutex;
use std::task::Waker;

use super::{InlineWaker, Readiness};
use crate::utils;

/// A collection of wakers which delegate to an in-line waker.
pub(crate) struct WakerList {
    wakers: Vec<Waker>,
    readiness: Arc<Mutex<Readiness>>,
}

impl WakerList {
    /// Create a new instance of `WakerList`.
    pub(crate) fn new(len: usize) -> Self {
        let readiness = Arc::new(Mutex::new(Readiness::new(len)));
        let wakers = (0..len)
            .map(|i| Arc::new(InlineWaker::new(i, readiness.clone())).into())
            .collect();
        Self { wakers, readiness }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&Waker> {
        self.wakers.get(index)
    }

    /// Access the `Readiness`.
    pub(crate) fn readiness(&self) -> &Mutex<Readiness> {
        self.readiness.as_ref()
    }
}
