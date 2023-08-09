use futures_core::Stream;
use slab::Slab;
use std::fmt::{self, Debug};
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

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
#[derive(Default)]
#[pin_project::pin_project]
pub struct FutureSet<T> {
    #[pin]
    futures: Slab<Pin<Box<dyn Future<Output = T>>>>,
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
            futures: Slab::with_capacity(capacity),
        }
    }

    /// Return the number of futures currently active in the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureSet;
    ///
    /// let mut set = FutureSet::with_capacity(2);
    /// assert_eq!(set.len(), 0);
    /// set.insert(async { 12 });
    /// assert_eq!(set.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.futures.len()
    }

    /// Returns true if there are no futures currently active in the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureSet;
    ///
    /// let mut set = FutureSet::with_capacity(2);
    /// assert!(set.is_empty());
    /// set.insert(async { 12 });
    /// assert!(!set.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.futures.is_empty()
    }

    /// Insert a new future into the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureSet;
    ///
    /// let mut set = FutureSet::with_capacity(2);
    /// set.insert(async { 12 });
    /// ```
    pub fn insert<Fut>(&mut self, fut: Fut)
    where
        Fut: Future<Output = T> + 'static,
    {
        self.futures.insert(Box::pin(fut));
    }
}
impl<T> Stream for FutureSet<T> {
    type Item = T;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        // Short-circuit if we have no futures to iterate over
        if this.futures.is_empty() {
            return Poll::Ready(None);
        }
        for (index, future) in this.futures.iter_mut() {
            match Pin::new(future).poll(cx) {
                std::task::Poll::Ready(item) => {
                    // A future resolved. Remove it from the set, and return its value.
                    this.futures.remove(index);
                    return Poll::Ready(Some(item));
                }
                std::task::Poll::Pending => continue,
            };
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod test {
    use super::FutureSet;
    // use async_iterator::LendingIterator;
    use futures_lite::prelude::*;
    // use std::pin::Pin;

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

    // // Wait for a future which resolves to a number, and a channel which
    // // receives a new future. When the future is received, we put it inside the
    // // set to resolve to a number.
    // #[test]
    // fn concurrent_channel() {
    //     enum Message<T> {
    //         Future(Pin<Box<dyn Future<Output = T> + 'static>>),
    //         Output(T),
    //     }
    //     futures_lite::future::block_on(async {
    //         let mut set = FutureSet::new();

    //         let (sender, receiver) = async_channel::bounded(1);
    //         sender.try_send(async { 2 + 2 }).unwrap();

    //         set.insert(async { Message::Output(1 + 1) });
    //         set.insert(async move {
    //             let fut = receiver.recv().await.unwrap();
    //             Message::Future(Box::pin(fut))
    //         });

    //         let mut out = 0;
    //         while let Some((msg, set)) = set.next().await {
    //             match msg {
    //                 Message::Future(fut) => set.insert(async move {
    //                     let output = fut.await;
    //                     Message::Output(output)
    //                 }),
    //                 Message::Output(num) => out += num,
    //             }
    //         }

    //         assert_eq!(out, 6);
    //     });
    // }
}
