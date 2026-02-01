#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;

use alloc::sync::{Arc, Weak};
use core::task::Waker;
use std::sync::{Mutex, MutexGuard};

use crate::utils::wakers::shared_arc::{waker_from_redirec_position, SharedArcContent};

use super::ReadinessVec;

/// A collection of wakers which delegate to an in-line waker.
pub(crate) struct WakerVec {
    wakers: Vec<Waker>,
    inner: Arc<WakerVecInner>,
}

struct WakerVecInner {
    redirect: Mutex<Vec<WakerVecInnerPtr>>,
    readiness: Mutex<ReadinessVec>,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct WakerVecInnerPtr(*const WakerVecInner);

unsafe impl Send for WakerVecInnerPtr {}
unsafe impl Sync for WakerVecInnerPtr {}

impl Default for WakerVec {
    fn default() -> Self {
        Self::new(0)
    }
}

impl WakerVec {
    /// Create a new instance of `WakerVec`.
    pub(crate) fn new(len: usize) -> Self {
        let inner = Arc::new_cyclic(|weak| {
            let raw = Weak::as_ptr(weak);
            WakerVecInner {
                redirect: Mutex::new(vec![WakerVecInnerPtr(raw); len]),
                readiness: Mutex::new(ReadinessVec::new(len)),
            }
        });
        let wakers = (0..len)
            .map(|i| unsafe { waker_from_redirec_position(Arc::clone(&inner), i) })
            .collect();
        Self { wakers, inner }
    }

    pub(crate) fn get(&self, index: usize) -> Option<&Waker> {
        self.wakers.get(index)
    }

    /// Access the `Readiness`.
    pub(crate) fn readiness(&self) -> MutexGuard<'_, ReadinessVec> {
        self.inner.readiness.lock().unwrap()
    }

    /// Resize the `WakerVec` to the new size.
    pub(crate) fn resize(&mut self, len: usize) {
        // If we grow the vec we'll need to extend beyond the current index.
        // Which means the first position is the current length, and every position
        // beyond that is incremented by 1.
        let mut index = self.wakers.len();

        let ptr = WakerVecInnerPtr(Arc::as_ptr(&self.inner));
        let mut lock = self.inner.redirect.lock().unwrap();
        lock.resize_with(len, || ptr);
        drop(lock);

        self.wakers.resize_with(len, || {
            let ret = unsafe { waker_from_redirec_position(Arc::clone(&self.inner), index) };
            index += 1;
            ret
        });

        let mut readiness = self.inner.readiness.lock().unwrap();
        readiness.resize(len);
    }
}

unsafe impl SharedArcContent for WakerVecInner {
    fn get_redirect_slice(&self) -> &[*const Self] {
        let slice = self.redirect.lock().unwrap();
        let slice = slice.as_slice();
        unsafe { core::mem::transmute(slice) }
    }

    fn wake_index(&self, index: usize) {
        let mut readiness = self.readiness.lock().unwrap();
        if !readiness.set_ready(index) {
            readiness
                .parent_waker()
                .as_ref()
                .expect("msg") // todo message
                .wake_by_ref()
        }
    }
}
