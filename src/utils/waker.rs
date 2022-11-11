use crate::stream::IntoStream;
use crate::utils::{self, Fuse, RandomGenerator};

use bitvec::bitvec;
use bitvec::vec::BitVec;
use core::fmt;
use futures_core::Stream;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

#[derive(Debug)]
pub(crate) struct Readiness {
    count: usize,
    ready: BitVec,
}

impl Readiness {
    /// Create a new instance of readiness.
    pub(crate) fn new(count: usize) -> Self {
        Self {
            count,
            ready: bitvec![true as usize; count],
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

    pub(crate) fn any_ready(&self) -> bool {
        self.count > 0
    }
}

/// An efficient waker which delegates wake events.
#[derive(Debug, Clone)]
pub(crate) struct InlineWaker {
    id: usize,
    readiness: Arc<Mutex<Readiness>>,
    parent_waker: Waker,
}

impl InlineWaker {
    pub(crate) fn new(id: usize, readiness: Arc<Mutex<Readiness>>, parent_waker: Waker) -> Self {
        Self {
            id,
            readiness,
            parent_waker,
        }
    }
}

impl Wake for InlineWaker {
    fn wake(self: std::sync::Arc<Self>) {
        if !self.readiness.lock().unwrap().set_ready(self.id) {
            self.parent_waker.wake_by_ref()
        }
    }
}

/// A collection of wakers.
pub(crate) struct WakerList {
    wakers: Vec<Waker>,
    has_parent: bool,
    readiness: Arc<Mutex<Readiness>>,
    len: usize,
}

impl WakerList {
    pub(crate) fn new(len: usize) -> Self {
        Self {
            has_parent: false,
            wakers: vec![],
            readiness: Arc::new(Mutex::new(Readiness::new(len))),
            len,
        }
    }

    pub(crate) fn has_parent(&self) -> bool {
        self.has_parent
    }

    pub(crate) fn set_parent(&mut self, parent: &Waker) {
        self.wakers = (0..self.len)
            .map(|i| Arc::new(InlineWaker::new(i, self.readiness.clone(), parent.clone())).into())
            .collect();

        self.has_parent = true;
    }

    pub(crate) fn get(&self, index: usize) -> Option<&Waker> {
        debug_assert!(
            self.has_parent,
            "no parent waker set. Did you forget to call `WakerList::set_parent?"
        );
        self.wakers.get(index)
    }

    pub(crate) fn readiness(&self) -> &Mutex<Readiness> {
        self.readiness.as_ref()
    }
}
