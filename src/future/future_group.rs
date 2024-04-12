use alloc::collections::BTreeSet;
use core::fmt::{self, Debug};
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::stream::Stream;
use futures_core::Future;
use slab::Slab;

use crate::utils::{PollState, PollVec, WakerVec};

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
#[derive(Default)]
#[pin_project::pin_project]
pub struct FutureGroup<F> {
    #[pin]
    futures: Slab<F>,
    wakers: WakerVec,
    states: PollVec,
    keys: BTreeSet<usize>,
    capacity: usize,
}

impl<T: Debug> Debug for FutureGroup<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FutureGroup")
            .field("slab", &"[..]")
            .field("len", &self.len())
            .field("capacity", &self.capacity)
            .finish()
    }
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
            futures: Slab::with_capacity(capacity),
            wakers: WakerVec::new(capacity),
            states: PollVec::new(capacity),
            keys: BTreeSet::new(),
            capacity,
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
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.futures.len()
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
        self.capacity
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
        self.futures.is_empty()
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
        let is_present = self.keys.remove(&key.0);
        if is_present {
            self.states[key.0].set_none();
            self.futures.remove(key.0);
        }
        is_present
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
        self.keys.contains(&key.0)
    }

    /// Reserves capacity for `additional` more futures to be inserted.
    /// Does nothing if the capacity is already sufficient.
    ///
    /// # Example
    ///
    /// ```rust
    /// use futures_concurrency::future::FutureGroup;
    /// use std::future::Ready;
    /// # futures_lite::future::block_on(async {
    /// let mut group: FutureGroup<Ready<usize>> = FutureGroup::with_capacity(0);
    /// assert_eq!(group.capacity(), 0);
    /// group.reserve(10);
    /// assert_eq!(group.capacity(), 10);
    ///
    /// // does nothing if capacity is sufficient
    /// group.reserve(5);
    /// assert_eq!(group.capacity(), 10);
    /// # })
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        if self.len() + additional < self.capacity {
            return;
        }
        let new_cap = self.capacity + additional;
        self.wakers.resize(new_cap);
        self.states.resize(new_cap);
        self.futures.reserve_exact(additional);
        self.capacity = new_cap;
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
        if self.capacity <= self.len() {
            self.reserve(self.capacity * 2 + 1);
        }

        let index = self.futures.insert(future);
        self.keys.insert(index);

        // Set the corresponding state
        self.states[index].set_pending();
        self.wakers.readiness().set_ready(index);

        Key(index)
    }

    /// Insert a value into a pinned `FutureGroup`
    ///
    /// This method is private because it serves as an implementation detail for
    /// `ConcurrentStream`. We should never expose this publicly, as the entire
    /// point of this crate is that we abstract the futures poll machinery away
    /// from end-users.
    pub(crate) fn insert_pinned(self: Pin<&mut Self>, future: F) -> Key
    where
        F: Future,
    {
        let mut this = self.project();
        // SAFETY: inserting a value into the futures slab does not ever move
        // any of the existing values.
        let index = unsafe { this.futures.as_mut().get_unchecked_mut() }.insert(future);
        this.keys.insert(index);
        let key = Key(index);

        // If our slab allocated more space we need to
        // update our tracking structures along with it.
        let max_len = this.futures.as_ref().capacity().max(index);
        this.wakers.resize(max_len);
        this.states.resize(max_len);

        // Set the corresponding state
        this.states[index].set_pending();
        let mut readiness = this.wakers.readiness();
        readiness.set_ready(index);

        key
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

        // Short-circuit if we have no futures to iterate over
        if this.futures.is_empty() {
            return Poll::Ready(None);
        }

        // Set the top-level waker and check readiness
        let mut readiness = this.wakers.readiness();
        readiness.set_waker(cx.waker());
        if !readiness.any_ready() {
            // Nothing is ready yet
            return Poll::Pending;
        }

        // Setup our futures state
        let mut ret = Poll::Pending;
        let states = this.states;

        // SAFETY: We unpin the future group so we can later individually access
        // single futures. Either to read from them or to drop them.
        let futures = unsafe { this.futures.as_mut().get_unchecked_mut() };

        for index in this.keys.iter().cloned() {
            if states[index].is_pending() && readiness.clear_ready(index) {
                // unlock readiness so we don't deadlock when polling
                #[allow(clippy::drop_non_drop)]
                drop(readiness);

                // Obtain the intermediate waker.
                let mut cx = Context::from_waker(this.wakers.get(index).unwrap());

                // SAFETY: this future here is a projection from the futures
                // vec, which we're reading from.
                let future = unsafe { Pin::new_unchecked(&mut futures[index]) };
                match future.poll(&mut cx) {
                    Poll::Ready(item) => {
                        // Set the return type for the function
                        ret = Poll::Ready(Some((Key(index), item)));

                        // Remove all associated data with the future
                        // The only data we can't remove directly is the key entry.
                        states[index] = PollState::None;
                        futures.remove(index);

                        break;
                    }
                    // Keep looping if there is nothing for us to do
                    Poll::Pending => {}
                };

                // Lock readiness so we can use it again
                readiness = this.wakers.readiness();
            }
        }

        // Now that we're no longer borrowing `this.keys` we can remove
        // the current key from the set
        if let Poll::Ready(Some((key, _))) = ret {
            this.keys.remove(&key.0);
        }

        ret
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

/// A key used to index into the `FutureGroup` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(usize);

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
            let mut group = FutureGroup::new();
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

    #[test]
    fn capacity_grow_on_insert() {
        futures_lite::future::block_on(async {
            let mut group = FutureGroup::new();
            let cap = group.capacity();

            group.insert(future::ready(1));

            assert!(group.capacity() > cap);
        });
    }
}
