use bitvec::{bitvec, vec::BitVec};
use std::task::Waker;

/// Tracks which wakers are "ready" and should be polled.
#[derive(Debug)]
pub(crate) struct ReadinessVec {
    count: usize,
    max_count: usize,
    ready: BitVec,
    parent_waker: Option<Waker>,
}

impl ReadinessVec {
    /// Create a new instance of readiness.
    pub(crate) fn new(count: usize) -> Self {
        Self {
            count,
            max_count: count,
            ready: bitvec![true as usize; count],
            parent_waker: None,
        }
    }

    /// Returns the old ready state for this id
    pub(crate) fn set_ready(&mut self, id: usize) -> bool {
        if !self.ready[id] {
            self.count += 1;
            self.ready.set(id, true);

            false
        } else {
            true
        }
    }

    /// Set all markers to ready.
    pub(crate) fn set_all_ready(&mut self) {
        self.ready.fill(true);
        self.count = self.max_count;
    }

    /// Returns whether the task id was previously ready
    pub(crate) fn clear_ready(&mut self, id: usize) -> bool {
        if self.ready[id] {
            self.count -= 1;
            self.ready.set(id, false);

            true
        } else {
            false
        }
    }

    /// Returns `true` if any of the wakers are ready.
    pub(crate) fn any_ready(&self) -> bool {
        self.count > 0
    }

    /// Access the parent waker.
    #[inline]
    pub(crate) fn parent_waker(&self) -> Option<&Waker> {
        self.parent_waker.as_ref()
    }

    /// Set the parent `Waker`. This needs to be called at the start of every
    /// `poll` function.
    pub(crate) fn set_waker(&mut self, parent_waker: &Waker) {
        self.parent_waker = Some(parent_waker.clone());
    }
}
