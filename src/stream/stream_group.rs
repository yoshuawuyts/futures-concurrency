use futures_core::Stream;
use slab::Slab;
use smallvec::{smallvec, SmallVec};
use std::collections::BTreeSet;
use std::fmt::{self, Debug};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::utils::{PollState, PollVec, WakerVec};

/// A growable group of streams which act as a single unit.
///
/// # Example
///
/// **Basic example**
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
/// while let Some(num) = group.next().await {
///     out += num;
/// }
/// assert_eq!(out, 6);
/// # });
/// ```
///
/// **Update the group on every iteration**
///
/// ```rust
/// use futures_concurrency::stream::StreamGroup;
/// use lending_stream::prelude::*;
/// use futures_lite::stream;
///
/// # futures_lite::future::block_on(async {
/// let mut group = StreamGroup::new();
/// group.insert(stream::once(4));

/// let mut index = 3;
/// let mut out = 0;
/// let mut group = group.lend_mut();
/// while let Some((group, num)) = group.next().await {
///     if index != 0 {
///         group.insert(stream::once(index));
///         index -= 1;
///     }
///     out += num;
/// }
/// assert_eq!(out, 10);
/// # });
/// ```
#[must_use = "`StreamGroup` does nothing if not iterated over"]
#[derive(Default)]
#[pin_project::pin_project]
pub struct StreamGroup<S> {
    #[pin]
    streams: Slab<S>,
    wakers: WakerVec,
    states: PollVec,
    keys: BTreeSet<usize>,
    key_removal_queue: SmallVec<[usize; 10]>,
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
            wakers: WakerVec::new(capacity),
            states: PollVec::new(capacity),
            keys: BTreeSet::new(),
            key_removal_queue: smallvec![],
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

    /// Return the capacity of the `StreamGroup`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::stream::StreamGroup;
    /// use futures_lite::stream;
    ///
    /// let group = StreamGroup::with_capacity(2);
    /// assert_eq!(group.capacity(), 2);
    /// # let group: StreamGroup<usize> = group;
    /// ```
    pub fn capacity(&self) -> usize {
        self.streams.capacity()
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
        let is_present = self.keys.remove(&key.0);
        if is_present {
            self.states[key.0].set_none();
            self.streams.remove(key.0);
        }
        is_present
    }

    /// Returns `true` if the `StreamGroup` contains a value for the specified key.
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
    /// assert!(group.contains_key(key));
    /// group.remove(key);
    /// assert!(!group.contains_key(key));
    /// # })
    /// ```
    pub fn contains_key(&mut self, key: Key) -> bool {
        self.keys.contains(&key.0)
    }
}

impl<S: Stream> StreamGroup<S> {
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
        self.keys.insert(index);
        let key = Key(index);

        // If our slab allocated more space we need to
        // update our tracking structures along with it.
        let max_len = self.capacity().max(index);
        self.wakers.resize(max_len);
        self.states.resize(max_len);

        // Set the corresponding state
        self.states[index].set_pending();
        let mut readiness = self.wakers.readiness().lock().unwrap();
        readiness.set_ready(index);

        key
    }

    /// Create a stream which also yields the key of each item.
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
    /// let mut group = group.keyed();
    /// while let Some((_key, num)) = group.next().await {
    ///     out += num;
    /// }
    /// assert_eq!(out, 6);
    /// # });
    /// ```
    pub fn keyed(self) -> Keyed<S> {
        Keyed { group: self }
    }
}

impl<S: Stream> StreamGroup<S> {
    fn poll_next_inner(
        self: Pin<&mut Self>,
        cx: &std::task::Context<'_>,
    ) -> Poll<Option<(Key, <S as Stream>::Item)>> {
        let mut this = self.project();

        // Short-circuit if we have no streams to iterate over
        if this.streams.is_empty() {
            return Poll::Ready(None);
        }

        // Set the top-level waker and check readiness
        let mut readiness = this.wakers.readiness().lock().unwrap();
        readiness.set_waker(cx.waker());
        if !readiness.any_ready() {
            // Nothing is ready yet
            return Poll::Pending;
        }

        // Setup our stream state
        let mut ret = Poll::Pending;
        let mut done_count = 0;
        let stream_count = this.streams.len();
        let states = this.states;

        // SAFETY: We unpin the stream set so we can later individually access
        // single streams. Either to read from them or to drop them.
        let streams = unsafe { this.streams.as_mut().get_unchecked_mut() };

        for index in this.keys.iter().cloned() {
            if states[index].is_pending() && readiness.clear_ready(index) {
                // unlock readiness so we don't deadlock when polling
                drop(readiness);

                // Obtain the intermediate waker.
                let mut cx = Context::from_waker(this.wakers.get(index).unwrap());

                // SAFETY: this stream here is a projection from the streams
                // vec, which we're reading from.
                let stream = unsafe { Pin::new_unchecked(&mut streams[index]) };
                match stream.poll_next(&mut cx) {
                    Poll::Ready(Some(item)) => {
                        // Set the return type for the function
                        ret = Poll::Ready(Some((Key(index), item)));

                        // We just obtained an item from this index, make sure
                        // we check it again on a next iteration
                        states[index] = PollState::Pending;
                        let mut readiness = this.wakers.readiness().lock().unwrap();
                        readiness.set_ready(index);

                        break;
                    }
                    Poll::Ready(None) => {
                        // A stream has ended, make note of that
                        done_count += 1;

                        // Remove all associated data about the stream.
                        // The only data we can't remove directly is the key entry.
                        states[index] = PollState::None;
                        streams.remove(index);
                        this.key_removal_queue.push(index);
                    }
                    // Keep looping if there is nothing for us to do
                    Poll::Pending => {}
                };

                // Lock readiness so we can use it again
                readiness = this.wakers.readiness().lock().unwrap();
            }
        }

        // Now that we're no longer borrowing `this.keys` we can loop over
        // which items we need to remove
        if !this.key_removal_queue.is_empty() {
            for key in this.key_removal_queue.iter() {
                this.keys.remove(key);
            }
            this.key_removal_queue.clear();
        }

        // If all streams turned up with `Poll::Ready(None)` our
        // stream should return that
        if done_count == stream_count {
            ret = Poll::Ready(None);
        }

        ret
    }
}

impl<S: Stream> Stream for StreamGroup<S> {
    type Item = <S as Stream>::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match self.poll_next_inner(cx) {
            Poll::Ready(Some((_key, item))) => Poll::Ready(Some(item)),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<S: Stream> FromIterator<S> for StreamGroup<S> {
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let len = iter.size_hint().1.unwrap_or_default();
        let mut this = Self::with_capacity(len);
        for stream in iter {
            this.insert(stream);
        }
        this
    }
}

/// A key used to index into the `StreamGroup` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(usize);

/// Iterate over items in the stream group with their associated keys.
#[derive(Debug)]
#[pin_project::pin_project]
pub struct Keyed<S: Stream> {
    #[pin]
    group: StreamGroup<S>,
}

impl<S: Stream> Deref for Keyed<S> {
    type Target = StreamGroup<S>;

    fn deref(&self) -> &Self::Target {
        &self.group
    }
}

impl<S: Stream> DerefMut for Keyed<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.group
    }
}

impl<S: Stream> Stream for Keyed<S> {
    type Item = (Key, <S as Stream>::Item);

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        this.group.as_mut().poll_next_inner(cx)
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
            while let Some(num) = group.next().await {
                out += num;
            }
            assert_eq!(out, 6);
            assert_eq!(group.len(), 0);
            assert!(group.is_empty());
        });
    }
}
