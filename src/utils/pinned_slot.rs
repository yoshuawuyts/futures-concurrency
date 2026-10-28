use core::{
    cell::UnsafeCell,
    fmt,
    future::Future,
    panic::RefUnwindSafe,
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;

/// The slot preserves suspended borrows during shared collection access.
///
/// Indexing and iteration can borrow a whole slice. The cell prevents shared
/// slice borrows from freezing the stored value.
#[repr(transparent)]
pub(crate) struct PinnedSlot<T>(UnsafeCell<T>);

impl<T> PinnedSlot<T> {
    pub(crate) fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    pub(crate) fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        // SAFETY: the value stays in the cell until it is dropped. The slot
        // provides no other access to the value after pinning.
        unsafe { self.map_unchecked_mut(|slot| slot.0.get_mut()) }
    }
}

// SAFETY: value access requires an exclusive borrow of the slot.
unsafe impl<T: Sync> Sync for PinnedSlot<T> {}

// Shared references to the slot cannot mutate its value.
impl<T: RefUnwindSafe> RefUnwindSafe for PinnedSlot<T> {}

impl<T> fmt::Debug for PinnedSlot<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Borrowing the value could invalidate a suspended mutable reference.
        formatter.debug_struct("PinnedSlot").finish_non_exhaustive()
    }
}

impl<T: Future> Future for PinnedSlot<T> {
    type Output = T::Output;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().poll(context)
    }
}

impl<T: Stream> Stream for PinnedSlot<T> {
    type Item = T::Item;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().poll_next(context)
    }
}
