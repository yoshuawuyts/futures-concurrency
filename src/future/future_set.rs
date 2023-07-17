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
/// This is returned by calling the [`handle`][`FutureSet::handle`] method on [`FutureSet`].
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
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureSet;
    ///
    /// let set = FutureSet::with_capacity(2);
    /// let handle = set.handle();
    ///
    /// assert_eq!(handle.len(), 0);
    /// handle.insert(async { 12 }).unwrap();
    /// assert_eq!(handle.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.futures.read().unwrap().len()
    }

    /// Returns true if there are no futures currently active in the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureSet;
    ///
    /// let set = FutureSet::with_capacity(2);
    /// let handle = set.handle();
    /// assert!(handle.is_empty());
    /// handle.insert(async { 12 }).unwrap();
    /// assert!(!handle.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.futures.read().unwrap().is_empty()
    }

    /// Insert a new future into the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureSet;
    ///
    /// let set = FutureSet::with_capacity(2);
    /// let handle = set.handle();
    /// handle.insert(async { 12 }).unwrap();
    /// ```
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
///
/// # Example
///
/// ```rust
/// use futures_concurrency::future::FutureSet;
/// use futures_lite::StreamExt;
///
/// # futures_lite::future::block_on(async {
/// let mut set = FutureSet::new();
/// set.insert(async { 5 });
/// set.insert(async { 7 });
///
/// let mut out = 0;
/// while let Some(num) = set.next().await {
///     out += num;
/// }
/// assert_eq!(out, 12);
/// # });
/// ```
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
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureSet;
    ///
    /// let set = FutureSet::new();
    /// # let set: FutureSet<usize> = set;
    /// ```
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new instance of `FutureSet` with a given capacity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureSet;
    ///
    /// let set = FutureSet::with_capacity(2);
    /// # let set: FutureSet<usize> = set;
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(true)),
            futures: Arc::new(RwLock::new(Slab::with_capacity(capacity))),
        }
    }

    /// Return the number of futures currently active in the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureSet;
    ///
    /// let set = FutureSet::with_capacity(2);
    /// assert_eq!(set.len(), 0);
    /// set.insert(async { 12 });
    /// assert_eq!(set.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.futures.read().unwrap().len()
    }

    /// Returns true if there are no futures currently active in the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureSet;
    ///
    /// let set = FutureSet::with_capacity(2);
    /// assert!(set.is_empty());
    /// set.insert(async { 12 });
    /// assert!(!set.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.futures.read().unwrap().is_empty()
    }

    /// Insert a new future into the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureSet;
    ///
    /// let set = FutureSet::with_capacity(2);
    /// set.insert(async { 12 });
    /// ```
    pub fn insert<Fut>(&self, fut: Fut)
    where
        Fut: Future<Output = T> + 'static,
    {
        self.futures.write().unwrap().insert(Box::pin(fut));
    }

    /// Obtain a handle to the set which can be used to insert more futures into the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureSet;
    ///
    /// let set = FutureSet::with_capacity(2);
    /// let handle = set.handle();
    /// handle.insert(async { 12 }).unwrap();
    /// ```
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
        this.is_active.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures_lite::prelude::*;

    #[test]
    fn smoke() {
        futures_lite::future::block_on(async {
            let mut set = FutureSet::new();
            set.insert(async { 1 + 1 });
            set.insert(async { 2 + 2 });

            let mut out = 0;
            while let Some(num) = set.next().await {
                out += num;
            }
            assert_eq!(out, 6);
            assert_eq!(set.len(), 0);
            assert!(set.is_empty());
        });
    }

    #[test]
    fn handle_works() {
        futures_lite::future::block_on(async {
            let mut set = FutureSet::new();
            let handle = set.handle();
            set.insert(async { 1 + 1 });
            handle.insert(async { 2 + 2 }).unwrap();

            let mut out = 0;
            while let Some(num) = set.next().await {
                out += num;
            }
            assert_eq!(out, 6);
        });
    }

    #[test]
    fn handle_errors_if_dropped() {
        futures_lite::future::block_on(async {
            let set = FutureSet::new();
            let handle = set.handle();
            drop(set);
            assert!(handle.insert(async {}).is_err());
        });
    }
}
