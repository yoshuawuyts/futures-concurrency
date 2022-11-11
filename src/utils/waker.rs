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

#[derive(Debug, Clone)]
pub(crate) struct StreamWaker {
    id: usize,
    readiness: Arc<Mutex<Readiness>>,
    parent_waker: Option<Waker>,
}

impl StreamWaker {
    pub(crate) fn new(id: usize, readiness: Arc<Mutex<Readiness>>) -> Self {
        Self {
            id,
            readiness,
            parent_waker: None,
        }
    }

    pub(crate) fn set_parent_waker(&mut self, parent: Waker) {
        self.parent_waker = Some(parent);
    }
}

impl Wake for StreamWaker {
    fn wake(self: std::sync::Arc<Self>) {
        if !self.readiness.lock().unwrap().set_ready(self.id) {
            let parent = self.parent_waker.as_ref().expect("No parent waker was set");
            parent.wake_by_ref()
        }
    }
}
