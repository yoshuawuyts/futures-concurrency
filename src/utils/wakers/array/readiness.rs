use bitvec::{bitvec, vec::BitVec};
use std::task::Waker;

use crate::utils;

/// Tracks which wakers are "ready" and should be polled.
#[derive(Debug)]
pub(crate) struct ReadinessArray<const N: usize> {
    count: usize,
    ready: [bool; N],
    parent_waker: Option<Waker>,
}

impl<const N: usize> ReadinessArray<N> {
    /// Create a new instance of readiness.
    pub(crate) fn new() -> Self {
        Self {
            count: N,
            ready: [true; N], // TODO: use a bitarray instead
            parent_waker: None,
        }
    }

    /// Returns the old ready state for this id
    pub(crate) fn set_ready(&mut self, id: usize) -> bool {
        if !self.ready[id] {
            self.count += 1;
            self.ready[id] = true;

            false
        } else {
            true
        }
    }

    /// Returns whether the task id was previously ready
    pub(crate) fn clear_ready(&mut self, id: usize) -> bool {
        if self.ready[id] {
            self.count -= 1;
            self.ready[id] = false;

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
