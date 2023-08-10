use futures_core::Stream;
use slab::Slab;
use std::fmt::{self, Debug};
use std::pin::Pin;
use std::task::Poll;

use crate::utils::iter_pin_mut_slab;

/// A growable group of streams which act as a single unit.
///
/// # Example
///
/// ```rust
/// use futures_concurrency::stream::StreamGroup;
/// use futures_lite::{stream, StreamExt};
///
/// # futures_lite::future::block_on(async {
/// let mut group = StreamGroup::new();
/// group.insert(stream::once(2));
/// group.insert(stream::once(4));
///
/// let mut out = 0;
/// while let Some((_key, num)) = group.next().await {
///     out += num;
/// }
/// assert_eq!(out, 6);
/// # });
/// ```
#[must_use = "`StreamGroup` does nothing if not iterated over"]
#[derive(Default)]
#[pin_project::pin_project]
pub struct StreamGroup<S> {
    #[pin]
    streams: Slab<S>,
}

impl<T: Debug> Debug for StreamGroup<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamGroup")
            .field("slab", &"[..]")
            .finish()
    }
}

impl<S> StreamGroup<S> {
    /// Create a new instance of `StreamGroup`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::stream::StreamGroup;
    ///
    /// let group = StreamGroup::new();
    /// # let group: StreamGroup<usize> = group;
    /// ```
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new instance of `StreamGroup` with a given capacity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::stream::StreamGroup;
    ///
    /// let group = StreamGroup::with_capacity(2);
    /// # let group: StreamGroup<usize> = group;
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            streams: Slab::with_capacity(capacity),
        }
    }

    /// Return the number of futures currently active in the group.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::stream::StreamGroup;
    /// use futures_lite::stream;
    ///
    /// let mut group = StreamGroup::with_capacity(2);
    /// assert_eq!(group.len(), 0);
    /// group.insert(stream::once(12));
    /// assert_eq!(group.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.streams.len()
    }

    /// Returns true if there are no futures currently active in the group.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::stream::StreamGroup;
    /// use futures_lite::stream;
    ///
    /// let mut group = StreamGroup::with_capacity(2);
    /// assert!(group.is_empty());
    /// group.insert(stream::once(12));
    /// assert!(!group.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.streams.is_empty()
    }

    /// Insert a new future into the group.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::stream::StreamGroup;
    /// use futures_lite::stream;
    ///
    /// let mut group = StreamGroup::with_capacity(2);
    /// group.insert(stream::once(12));
    /// ```
    pub fn insert(&mut self, stream: S) -> Key
    where
        S: Stream,
    {
        let index = self.streams.insert(stream);
        Key(index)
    }

    /// Removes a stream from the group. Returns whether the value was present in
    /// the group.
    ///
    /// # Example
    ///
    /// ```
    /// use futures_lite::stream;
    /// use futures_concurrency::stream::StreamGroup;
    ///
    /// # futures_lite::future::block_on(async {
    /// let mut group = StreamGroup::new();
    /// let key = group.insert(stream::once(4));
    /// assert_eq!(group.len(), 1);
    /// group.remove(key);
    /// assert_eq!(group.len(), 0);
    /// # })
    /// ```
    pub fn remove(&mut self, key: Key) -> bool {
        match self.streams.try_remove(key.0) {
            Some(_) => true,
            None => false,
        }
    }
}

/// A key used to index into the `StreamGroup` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(usize);

impl<S: Stream> Stream for StreamGroup<S> {
    type Item = (Key, <S as Stream>::Item);

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
        // we should replace this with a proper `Wakergroup`
        let mut remove_idx = vec![];
        let mut ret = Poll::Pending;
        let total = this.streams.len();
        let mut empty = 0;

        for (index, stream) in iter_pin_mut_slab(this.streams.as_mut()) {
            match stream.poll_next(cx) {
                Poll::Ready(Some(item)) => {
                    ret = Poll::Ready(Some((Key(index), item)));
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
    use super::StreamGroup;
    use futures_lite::{prelude::*, stream};

    #[test]
    fn smoke() {
        futures_lite::future::block_on(async {
            let mut group = StreamGroup::new();
            group.insert(stream::once(2));
            group.insert(stream::once(4));

            let mut out = 0;
            while let Some((_key, num)) = group.next().await {
                out += num;
            }
            assert_eq!(out, 6);
            assert_eq!(group.len(), 0);
            assert!(group.is_empty());
        });
    }
}
