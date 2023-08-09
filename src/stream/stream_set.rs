use futures_core::Stream;
use slab::Slab;
use std::borrow::Borrow;
use std::fmt::{self, Debug};
use std::pin::Pin;
use std::task::Poll;

use crate::utils::iter_pin_mut_slab;

/// A dynamic set of streams.
///
/// # Example
///
/// ```rust
/// use futures_concurrency::stream::StreamSet;
/// use futures_lite::StreamExt;
///
/// # futures_lite::future::block_on(async {
/// # });
/// ```
#[must_use = "`StreamSet` does nothing if not iterated over"]
#[derive(Default)]
#[pin_project::pin_project]
pub struct StreamSet<S> {
    #[pin]
    streams: Slab<S>,
}

impl<T: Debug> Debug for StreamSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamSet").field("slab", &"[..]").finish()
    }
}

impl<S> StreamSet<S> {
    /// Create a new instance of `StreamSet`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::stream::StreamSet;
    ///
    /// let set = StreamSet::new();
    /// # let set: StreamSet<usize> = set;
    /// ```
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new instance of `StreamSet` with a given capacity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::stream::StreamSet;
    ///
    /// let set = StreamSet::with_capacity(2);
    /// # let set: StreamSet<usize> = set;
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            streams: Slab::with_capacity(capacity),
        }
    }

    /// Return the number of futures currently active in the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::stream::StreamSet;
    /// use futures_lite::stream;
    ///
    /// let mut set = StreamSet::with_capacity(2);
    /// assert_eq!(set.len(), 0);
    /// set.insert(stream::once(async { 12 }));
    /// assert_eq!(set.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.streams.len()
    }

    /// Returns true if there are no futures currently active in the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::stream::StreamSet;
    /// use futures_lite::stream;
    ///
    /// let mut set = StreamSet::with_capacity(2);
    /// assert!(set.is_empty());
    /// set.insert(stream::once(async { 12 }));
    /// assert!(!set.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.streams.is_empty()
    }

    /// Insert a new future into the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::stream::StreamSet;
    /// use futures_lite::stream;
    ///
    /// let mut set = StreamSet::with_capacity(2);
    /// set.insert(stream::once(async { 12 }));
    /// ```
    pub fn insert(&mut self, stream: S)
    where
        S: Stream,
    {
        self.streams.insert(stream);
    }

    /// Removes a stream from the set. Returns whether the value was present in
    /// the set.
    ///
    /// Note that streams are automatically removed from the set once they yield
    /// `None`. This method should only be used to remove streams which have not
    /// yet completed.
    ///
    /// The value may be any borrowed form of the set’s value type, but Eq on
    /// the borrowed form must match those for the value type.
    pub fn remove<Q>(&mut self, k: &Q) -> bool
    where
        S: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        for i in 0..self.streams.len() {
            if self.streams[i].borrow() == k {
                self.streams.remove(i);
                return true;
            }
        }

        false
    }
}

impl<S: Stream> Stream for StreamSet<S> {
    type Item = <S as Stream>::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Short-circuit if we have no streams to iterate over
        if this.streams.is_empty() {
            return Poll::Ready(None);
        }

        // NOTE(yosh): lol, this is so bad
        // we should replace this with a proper `WakerSet`
        let mut remove_idx = vec![];
        let mut ret = Poll::Pending;
        let total = this.streams.len();
        let mut empty = 0;

        for (index, stream) in iter_pin_mut_slab(this.streams.as_mut()) {
            match stream.poll_next(cx) {
                Poll::Ready(Some(item)) => {
                    ret = Poll::Ready(Some(item));
                    break;
                }
                Poll::Ready(None) => {
                    // The stream ended, remove the item.
                    remove_idx.push(index);
                    empty += 1;
                    continue;
                }
                Poll::Pending => continue,
            };
        }

        // Remove all indexes we just flagged as ready for removal
        for idx in remove_idx {
            // SAFETY: we're accessing the internal `streams` store
            // only to drop the stream.
            let streams = unsafe { this.streams.as_mut().get_unchecked_mut() };
            streams.remove(idx);
        }

        // If all streams turned up with `Poll::Ready(None)` our
        // stream should return that
        if empty == total {
            ret = Poll::Ready(None);
        }

        ret
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Borrow;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use super::StreamSet;
    use futures_lite::{prelude::*, stream};

    #[test]
    fn smoke() {
        futures_lite::future::block_on(async {
            let mut set = StreamSet::new();
            set.insert(stream::once(2));
            set.insert(stream::once(4));

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
    fn remove() {
        futures_lite::future::block_on(async {
            use futures_lite::stream::{once, Once};

            // Create a wrapper struct which holds a stream and an index
            type Key = u64;
            struct Wrapper(Once<usize>, Key);

            // We only want to compare the keys
            impl Eq for Wrapper {}
            impl PartialEq for Wrapper {
                fn eq(&self, other: &Self) -> bool {
                    self.1 == other.1
                }
            }

            // Borrowing the stream should provide us with a key
            impl Borrow<Key> for Wrapper {
                fn borrow(&self) -> &Key {
                    &self.1
                }
            }

            // Forward the stream implementation
            impl Stream for Wrapper {
                type Item = usize;
                fn poll_next(
                    mut self: Pin<&mut Self>,
                    cx: &mut Context<'_>,
                ) -> Poll<Option<Self::Item>> {
                    self.0.poll_next(cx)
                }
            }

            // We're now all-ready to start using the wrapper!
            let key = 1;
            let stream = Wrapper(once(12), key);
            let mut set = StreamSet::new();
            set.insert(stream);
            assert_eq!(set.len(), 1);
            set.remove(&key);
            assert_eq!(set.len(), 0);
        })
    }
}
