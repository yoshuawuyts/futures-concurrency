#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;

use core::{mem::ManuallyDrop, pin::Pin};

use crate::utils::PinnedSlot;

/// The vector stores futures for polling and destruction in place.
pub(crate) struct FutureVec<T> {
    futures: Vec<PinnedSlot<ManuallyDrop<T>>>,
}

impl<T> FutureVec<T> {
    pub(crate) fn new(futures: Vec<T>) -> Self {
        Self {
            futures: futures
                .into_iter()
                .map(|future| PinnedSlot::new(ManuallyDrop::new(future)))
                .collect(),
        }
    }

    pub(crate) fn iter(self: Pin<&mut Self>) -> impl Iterator<Item = Pin<&mut ManuallyDrop<T>>> {
        // SAFETY: the vector never reallocates or moves its elements after pinning.
        unsafe { self.get_unchecked_mut() }
            .futures
            .iter_mut()
            .map(|slot| unsafe { Pin::new_unchecked(slot) }.project())
    }

    /// Destruction preserves the indexed future's address.
    ///
    /// # Safety
    ///
    /// The indexed future must be initialized. The caller must prevent later
    /// access to the future and must not drop it again.
    pub(crate) unsafe fn drop(self: Pin<&mut Self>, index: usize) {
        let future = self.iter().nth(index).expect("future index is in bounds");
        // SAFETY: the caller guarantees exclusive destruction of this live future.
        unsafe { ManuallyDrop::drop(future.get_unchecked_mut()) };
    }
}
