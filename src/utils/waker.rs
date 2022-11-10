use crate::stream::IntoStream;
use crate::utils::{self, Fuse, RandomGenerator};

use core::fmt;
use futures_core::Stream;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

pub(crate) struct Readiness {
    count: usize,
    ready: Vec<bool>, // TODO: Use a bitvector
}

impl Readiness {
    /// Create a new instance of reaciness.
    pub(crate) fn new(count: usize) -> Self {
        Self {
            count,
            ready: vec![true; count],
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

    pub(crate) fn any_ready(&self) -> bool {
        self.count > 0
    }
}

pub(crate) struct StreamWaker {
    id: usize,
    readiness: Arc<Mutex<Readiness>>,
    parent: Waker,
}

impl StreamWaker {
    pub(crate) fn new(id: usize, readiness: Arc<Mutex<Readiness>>, parent: Waker) -> Self {
        Self {
            id,
            readiness,
            parent,
        }
    }
}

impl Wake for StreamWaker {
    fn wake(self: std::sync::Arc<Self>) {
        if !self.readiness.lock().unwrap().set_ready(self.id) {
            self.parent.wake_by_ref()
        }
    }
}
