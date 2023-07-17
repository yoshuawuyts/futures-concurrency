use futures_core::Stream;
use pin_project::pin_project;
use slab::Slab;
use std::error;
use std::fmt::{self, Debug};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::task::Poll;

/// A handle to a dynamic set of futures.
///
/// See [`FutureSet`] for more.
#[derive(Clone)]
#[pin_project]
pub struct FutureSetHandle<T> {
    #[pin]
    futures: Arc<RwLock<Slab<Pin<Box<dyn Future<Output = T> + 'static>>>>>,
    is_active: Arc<AtomicBool>,
}

impl<T: Debug> Debug for FutureSetHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FutureSet").field("slab", &"[..]").finish()
    }
}

impl<T> FutureSetHandle<T> {
    /// Return the number of futures currently active in the set.
    pub fn len(&self) -> usize {
        self.futures.read().unwrap().len()
    }

    /// Returns true if there are no futures currently active in the set.
    pub fn is_empty(&self) -> bool {
        self.futures.read().unwrap().is_empty()
    }

    /// Insert a new future into the set.
    pub fn insert<Fut>(&self, fut: Fut) -> Result<(), InsertError>
    where
        Fut: Future<Output = T> + 'static,
    {
        if !self.is_active.load(Ordering::SeqCst) {
            return Err(InsertError { _sealed: () });
        }
        self.futures.write().unwrap().insert(Box::pin(fut));
        Ok(())
    }
}
/// An error returned from [`FutureSetHandle::insert()`].
///
/// Received because the underlying `FutureSet` no longer exists.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct InsertError {
    _sealed: (),
}

impl error::Error for InsertError {}

impl Debug for InsertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InsertError(..)")
    }
}
impl fmt::Display for InsertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "inserting on a `FutureSet` which no longer exists")
    }
}

/// A dynamic set of futures.
#[must_use = "`FutureSet` does nothing if not iterated over"]
#[pin_project(PinnedDrop)]
pub struct FutureSet<T> {
    #[pin]
    futures: Arc<RwLock<Slab<Pin<Box<dyn Future<Output = T> + 'static>>>>>,
    is_active: Arc<AtomicBool>,
}

impl<T: Debug> Debug for FutureSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FutureSet").field("slab", &"[..]").finish()
    }
}

impl<T> FutureSet<T> {
    /// Create a new instance of `FutureSet`.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new instance of `FutureSet` with a given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            futures: Arc::new(RwLock::new(Slab::with_capacity(capacity))),
        }
    }

    /// Return the number of futures currently active in the set.
    pub fn len(&self) -> usize {
        self.futures.read().unwrap().len()
    }

    /// Returns true if there are no futures currently active in the set.
    pub fn is_empty(&self) -> bool {
        self.futures.read().unwrap().is_empty()
    }

    /// Insert a new future into the set.
    pub fn insert<Fut>(&self, fut: Fut)
    where
        Fut: Future<Output = T> + 'static,
    {
        self.futures.write().unwrap().insert(Box::pin(fut));
    }

    /// Obtain a handle to the set which can be used to insert more futures into the set.
    pub fn handle(&self) -> FutureSetHandle<T> {
        FutureSetHandle {
            futures: self.futures.clone(),
            is_active: self.is_active.clone(),
        }
    }
}

impl<T> Stream for FutureSet<T> {
    type Item = T;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        let mut futures = this.futures.write().unwrap();

        // Short-circuit if we have no futures to iterate over
        if futures.is_empty() {
            return Poll::Ready(None);
        }

        for (index, future) in futures.iter_mut() {
            match Pin::new(future).poll(cx) {
                std::task::Poll::Ready(item) => {
                    // A future resolved. Remove it from the set, and return its value.
                    futures.remove(index);
                    return Poll::Ready(Some(item));
                }
                std::task::Poll::Pending => continue,
            };
        }
        Poll::Pending
    }
}

#[pin_project::pinned_drop]
impl<T> PinnedDrop for FutureSet<T> {
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();
        this.is_active.store(true, Ordering::SeqCst);
    }
}
