use core::fmt::Debug;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::Stream;

use crate::collections::inner_group::{InnerGroup, Key, PollStream};

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
#[derive(Default, Debug)]
#[pin_project::pin_project]
pub struct StreamGroup<S> {
    #[pin]
    inner: InnerGroup<S, PollStream>,
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
            inner: InnerGroup::with_capacity(capacity),
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
        self.inner.len()
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
        self.inner.capacity()
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
        self.inner.is_empty()
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
        self.inner.remove(key).is_some()
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
        self.inner.contains_key(key)
    }

    /// Reserves capacity for `additional` more streams to be inserted.
    /// Does nothing if the capacity is already sufficient.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::stream::StreamGroup;
    /// use futures_lite::stream::Once;
    /// # futures_lite::future::block_on(async {
    /// let mut group: StreamGroup<Once<usize>> = StreamGroup::with_capacity(0);
    /// assert_eq!(group.capacity(), 0);
    /// group.reserve(10);
    /// assert_eq!(group.capacity(), 10);
    /// # })
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
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
        self.inner.insert(stream)
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

impl<S: Stream> Stream for StreamGroup<S> {
    type Item = <S as Stream>::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        match this.inner.poll_next_inner(cx) {
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

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        // SAFETY: pin-projecting to the inner group is safe because we trust
        // it that it's correctly pinned
        let inner = unsafe { this.group.as_mut().map_unchecked_mut(|t| &mut t.inner) };
        inner.poll_next_inner(cx)
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
