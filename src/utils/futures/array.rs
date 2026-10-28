use core::{mem::ManuallyDrop, pin::Pin};

use crate::utils::PinnedSlot;

/// The array stores futures for polling and destruction in place.
pub(crate) struct FutureArray<T, const N: usize> {
    futures: [PinnedSlot<ManuallyDrop<T>>; N],
}

impl<T, const N: usize> FutureArray<T, N> {
    pub(crate) fn new(futures: [T; N]) -> Self {
        Self {
            futures: futures.map(|future| PinnedSlot::new(ManuallyDrop::new(future))),
        }
    }

    pub(crate) fn iter(self: Pin<&mut Self>) -> impl Iterator<Item = Pin<&mut ManuallyDrop<T>>> {
        // SAFETY: the array stays at its pinned address until it is dropped.
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
