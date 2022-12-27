use std::task::Waker;

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

    /// Set all markers to ready.
    pub(crate) fn set_all_ready(&mut self) {
        self.ready.fill(true);
        self.count = N;
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

    /// Set the parent `Waker`. This needs to be called at the start of every
    /// `poll` function.
    pub(crate) fn set_waker(&mut self, parent_waker: &Waker) {
        self.parent_waker = Some(parent_waker.clone());
    }

    pub(crate) fn wake(&mut self, index: usize) {
        if !self.set_ready(index) {
            self.parent_waker
                .as_ref()
                .expect("`parent_waker` not available from `Readiness`. Did you forget to call `Readiness::set_waker`?")
                .wake_by_ref()
        }
    }
}
