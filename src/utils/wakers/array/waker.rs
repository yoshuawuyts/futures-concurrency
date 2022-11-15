use crate::utils;
use std::sync;
use std::sync::Arc;
use std::sync::Mutex;
use std::task;
use std::task::Wake;
use std::task::Waker;

use super::ReadinessArray;

/// An efficient waker which delegates wake events.
#[derive(Debug, Clone)]
pub(crate) struct InlineWaker<const N: usize> {
    pub(crate) id: usize,
    pub(crate) readiness: Arc<Mutex<ReadinessArray<N>>>,
}

impl<const N: usize> InlineWaker<N> {
    /// Create a new instance of `InlineWaker`.
    pub(crate) fn new(id: usize, readiness: Arc<Mutex<ReadinessArray<N>>>) -> Self {
        Self { id, readiness }
    }
}

impl<const N: usize> Wake for InlineWaker<N> {
    fn wake(self: std::sync::Arc<Self>) {
        let mut readiness = self.readiness.lock().unwrap();
        if !readiness.set_ready(self.id) {
            readiness
                .parent_waker()
                .as_mut()
                .expect("`parent_waker` not available from `Readiness`. Did you forget to call `Readiness::set_waker`?")
                .wake_by_ref()
        }
    }
}
