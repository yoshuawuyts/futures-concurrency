use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::stream::Stream;
use futures_core::Future;

use crate::collections::group_inner::{GroupInner, Key};

/// A growable group of futures which act as a single unit.
///
/// # Example
///
/// **Basic example**
///
/// ```rust
/// use futures_concurrency::future::FutureGroup;
/// use futures_lite::StreamExt;
/// use std::future;
///
/// # futures_lite::future::block_on(async {
/// let mut group = FutureGroup::new();
/// group.insert(future::ready(2));
/// group.insert(future::ready(4));
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
/// ```
/// use futures_concurrency::future::FutureGroup;
/// use lending_stream::prelude::*;
/// use std::future;
///
/// # fn main() { futures_lite::future::block_on(async {
/// let mut group = FutureGroup::new();
/// group.insert(future::ready(4));
///
/// let mut index = 3;
/// let mut out = 0;
/// let mut group = group.lend_mut();
/// while let Some((group, num)) = group.next().await {
///     if index != 0 {
///         group.insert(future::ready(index));
///         index -= 1;
///     }
///     out += num;
/// }
/// assert_eq!(out, 10);
/// # });}
/// ```
#[must_use = "`FutureGroup` does nothing if not iterated over"]
#[derive(Default, Debug)]
#[pin_project::pin_project]
pub struct FutureGroup<F> {
    #[pin]
    inner: GroupInner<F>,
}

impl<F> FutureGroup<F> {
    /// Create a new instance of `FutureGroup`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureGroup;
    ///
    /// let group = FutureGroup::new();
    /// # let group: FutureGroup<usize> = group;
    /// ```
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new instance of `FutureGroup` with a given capacity.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureGroup;
    ///
    /// let group = FutureGroup::with_capacity(2);
    /// # let group: FutureGroup<usize> = group;
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: GroupInner::with_capacity(capacity),
        }
    }

    /// Return the number of futures currently active in the group.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureGroup;
    /// use futures_lite::StreamExt;
    /// use std::future;
    ///
    /// let mut group = FutureGroup::with_capacity(2);
    /// assert_eq!(group.len(), 0);
    /// group.insert(future::ready(12));
    /// assert_eq!(group.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Return the capacity of the `FutureGroup`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureGroup;
    /// use futures_lite::stream;
    ///
    /// let group = FutureGroup::with_capacity(2);
    /// assert_eq!(group.capacity(), 2);
    /// # let group: FutureGroup<usize> = group;
    /// ```
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Returns true if there are no futures currently active in the group.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureGroup;
    /// use std::future;
    ///
    /// let mut group = FutureGroup::with_capacity(2);
    /// assert!(group.is_empty());
    /// group.insert(future::ready(12));
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
    /// use futures_concurrency::future::FutureGroup;
    /// use std::future;
    ///
    /// # futures_lite::future::block_on(async {
    /// let mut group = FutureGroup::new();
    /// let key = group.insert(future::ready(4));
    /// assert_eq!(group.len(), 1);
    /// group.remove(key);
    /// assert_eq!(group.len(), 0);
    /// # })
    /// ```
    pub fn remove(&mut self, key: Key) -> bool {
        // TODO(consoli): is it useful to return the removed future here?
        self.inner.remove(key).is_some()
    }

    /// Returns `true` if the `FutureGroup` contains a value for the specified key.
    ///
    /// # Example
    ///
    /// ```
    /// use futures_concurrency::future::FutureGroup;
    /// use std::future;
    ///
    /// # futures_lite::future::block_on(async {
    /// let mut group = FutureGroup::new();
    /// let key = group.insert(future::ready(4));
    /// assert!(group.contains_key(key));
    /// group.remove(key);
    /// assert!(!group.contains_key(key));
    /// # })
    /// ```
    pub fn contains_key(&mut self, key: Key) -> bool {
        self.inner.contains_key(key)
    }
}

impl<F: Future> FutureGroup<F> {
    /// Insert a new future into the group.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureGroup;
    /// use std::future;
    ///
    /// let mut group = FutureGroup::with_capacity(2);
    /// group.insert(future::ready(12));
    /// ```
    pub fn insert(&mut self, future: F) -> Key
    where
        F: Future,
    {
        self.inner.insert(future)
    }

    /// Insert a value into a pinned `FutureGroup`
    ///
    /// This method is private because it serves as an implementation detail for
    /// `ConcurrentStream`. We should never expose this publicly, as the entire
    /// point of this crate is that we abstract the futures poll machinery away
    /// from end-users.
    pub(crate) fn insert_pinned(self: Pin<&mut Self>, future: F) -> Key {
        let this = self.project();
        this.inner.insert_pinned(future)

        // let inner = unsafe { self.as_mut().map_unchecked_mut(|this| &mut this.inner) };
        // inner.insert_pinned(future)
    }

    /// Create a stream which also yields the key of each item.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureGroup;
    /// use futures_lite::StreamExt;
    /// use std::future;
    ///
    /// # futures_lite::future::block_on(async {
    /// let mut group = FutureGroup::new();
    /// group.insert(future::ready(2));
    /// group.insert(future::ready(4));
    ///
    /// let mut out = 0;
    /// let mut group = group.keyed();
    /// while let Some((_key, num)) = group.next().await {
    ///     out += num;
    /// }
    /// assert_eq!(out, 6);
    /// # });
    /// ```
    pub fn keyed(self) -> Keyed<F> {
        Keyed { group: self }
    }
}

impl<F: Future> FutureGroup<F> {
    fn poll_next_inner(
        self: Pin<&mut Self>,
        cx: &Context<'_>,
    ) -> Poll<Option<(Key, <F as Future>::Output)>> {
        let mut this = self.project();
        let inner = unsafe { &mut this.inner.as_mut().get_unchecked_mut() };

        // Short-circuit if we have no futures to iterate over
        if inner.is_empty() {
            return Poll::Ready(None);
        }

        // Set the top-level waker and check readiness
        inner.set_top_waker(cx.waker());
        if !inner.any_ready() {
            // Nothing is ready yet
            return Poll::Pending;
        }

        for index in inner.keys.iter().cloned() {
            // verify if the `index`th future can be polled
            if !inner.can_progress_index(index) {
                continue;
            }

            // Obtain the intermediate waker.
            let mut cx = Context::from_waker(inner.wakers.get(index).unwrap());

            // SAFETY: this future here is a projection from the futures
            // vec, which we're reading from.
            let future = unsafe { Pin::new_unchecked(&mut inner.items[index]) };
            if let Poll::Ready(item) = future.poll(&mut cx) {
                let key = Key(index);
                // Set the return type for the function
                let ret = Poll::Ready(Some((key, item)));

                // Remove all associated data with the future
                // The only data we can't remove directly is the key entry.
                inner.remove(key);

                return ret;
            }
        }
        Poll::Pending
    }
}

impl<F: Future> Stream for FutureGroup<F> {
    type Item = <F as Future>::Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.poll_next_inner(cx) {
            Poll::Ready(Some((_key, item))) => Poll::Ready(Some(item)),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<F: Future> FromIterator<F> for FutureGroup<F> {
    fn from_iter<T: IntoIterator<Item = F>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let len = iter.size_hint().1.unwrap_or_default();
        let mut this = Self::with_capacity(len);
        for future in iter {
            this.insert(future);
        }
        this
    }
}

/// Iterate over items in the futures group with their associated keys.
#[derive(Debug)]
#[pin_project::pin_project]
pub struct Keyed<F: Future> {
    #[pin]
    group: FutureGroup<F>,
}

impl<F: Future> Deref for Keyed<F> {
    type Target = FutureGroup<F>;

    fn deref(&self) -> &Self::Target {
        &self.group
    }
}

impl<F: Future> DerefMut for Keyed<F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.group
    }
}

impl<F: Future> Stream for Keyed<F> {
    type Item = (Key, <F as Future>::Output);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        this.group.as_mut().poll_next_inner(cx)
    }
}

#[cfg(test)]
mod test {
    use super::FutureGroup;
    use core::future;
    use futures_lite::prelude::*;

    #[test]
    fn smoke() {
        futures_lite::future::block_on(async {
            let mut group = FutureGroup::with_capacity(0);
            group.insert(future::ready(2));
            group.insert(future::ready(4));

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
