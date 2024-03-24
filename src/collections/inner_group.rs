use core::{
    future::Future,
    marker::PhantomData,
    ops::ControlFlow,
    pin::Pin,
    task::{Context, Poll},
};

use alloc::{collections::BTreeSet, fmt};
use futures_core::Stream;
use slab::Slab;
use smallvec::{smallvec, SmallVec};

use crate::utils::{PollVec, WakerVec};

const fn grow_group_capacity(cap: usize) -> usize {
    cap * 2 + 1
}

#[pin_project::pin_project]
pub struct InnerGroup<A, B> {
    #[pin]
    items: Slab<A>,
    wakers: WakerVec,
    states: PollVec,
    keys: BTreeSet<usize>,
    cap: usize,
    len: usize,
    _poll_behavior: PhantomData<B>,
}

impl<A, B> InnerGroup<A, B> {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            items: Slab::with_capacity(cap),
            wakers: WakerVec::new(cap),
            states: PollVec::new(cap),
            keys: BTreeSet::new(),
            cap,
            len: 0,
            _poll_behavior: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn has_capacity(&self) -> bool {
        self.len < self.cap
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn insert(&mut self, item: A) -> Key {
        if !self.has_capacity() {
            self.reserve(grow_group_capacity(self.cap));
        }

        let index = self.items.insert(item);
        self.keys.insert(index);

        // set the corresponding state
        self.states[index].set_pending();
        self.wakers.readiness().set_ready(index);

        self.len += 1;
        Key(index)
    }

    pub fn insert_pinned(mut self: Pin<&mut Self>, item: A) -> Key {
        // SAFETY: inserting a value into the slab does not ever move
        // any of the existing values
        let this = unsafe { &mut self.as_mut().get_unchecked_mut() };
        this.insert(item)
    }

    pub fn remove(&mut self, key: Key) -> Option<A> {
        let is_present = self.keys.remove(&key.0);
        if !is_present {
            return None;
        }
        self.states[key.0].set_none();
        let item = self.items.remove(key.0);
        self.len -= 1;
        Some(item)
    }

    /// Reserve `additional` capacity for new items
    /// Does nothing if the capacity is already sufficient
    pub fn reserve(&mut self, additional: usize) {
        if self.len + additional < self.cap {
            return;
        }
        let new_cap = self.cap + additional;
        self.wakers.resize(new_cap);
        self.states.resize(new_cap);
        self.items.reserve_exact(new_cap);
        self.cap = new_cap;
    }

    // move to other impl block
    pub fn contains_key(&self, key: Key) -> bool {
        self.items.contains(key.0)
    }
}

impl<A, B> Default for InnerGroup<A, B> {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

impl<A, B> fmt::Debug for InnerGroup<A, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InnerGroup")
            .field("cap", &self.cap)
            .field("len", &self.len)
            .finish()
    }
}

/// A key used to index into the `FutureGroup` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(pub usize);

impl<A, B> InnerGroup<A, B>
where
    B: PollBehavior<A>,
{
    pub fn poll_next_inner(
        mut self: Pin<&mut Self>,
        cx: &Context<'_>,
    ) -> Poll<Option<(Key, B::Output)>> {
        // short-circuit if we have no items to iterate over
        if self.is_empty() {
            return Poll::Ready(None);
        }

        // SAFETY: inserting and removing items from the slab does not ever
        // move any of the existing values
        let this = unsafe { self.as_mut().get_unchecked_mut() };

        // set the top-level waker and check readiness
        this.wakers.readiness().set_waker(cx.waker());
        if !this.wakers.readiness().any_ready() {
            // nothing is ready yet
            return Poll::Pending;
        }

        let mut done_count = 0;
        let group_len = this.len();
        let mut removal_queue: SmallVec<[_; 10]> = smallvec![];

        let mut ret = Poll::Pending;

        for index in this.keys.iter().cloned() {
            // can we make progress for this item?
            if !(this.states[index].is_pending() && this.wakers.readiness().clear_ready(index)) {
                continue;
            }

            // obtain the intermediate waker
            let mut cx = Context::from_waker(this.wakers.get(index).unwrap());

            // SAFETY: this item here is a projection from the slab, which we're reading from
            let pollable = unsafe { Pin::new_unchecked(&mut this.items[index]) };
            match B::poll(pollable, &mut cx) {
                ControlFlow::Break((result, PollAgain::Stop)) => {
                    for item in removal_queue {
                        this.remove(item);
                    }
                    this.remove(Key(index));
                    return Poll::Ready(Some((Key(index), result)));
                }
                ControlFlow::Break((result, PollAgain::Poll)) => {
                    this.states[index].set_pending();
                    this.wakers.readiness().set_ready(index);

                    ret = Poll::Ready(Some((Key(index), result)));
                    break;
                }
                ControlFlow::Continue(PollAgain::Stop) => {
                    done_count += 1;
                    removal_queue.push(Key(index));
                    continue;
                }
                ControlFlow::Continue(PollAgain::Poll) => continue,
            }
        }
        for item in removal_queue {
            this.remove(item);
        }

        if done_count == group_len {
            return Poll::Ready(None);
        }

        ret
    }
}

/// Used to tell the callee of [`PollBehavior::poll`]
/// whether the `poll`'ed item should be polled again.
pub(crate) enum PollAgain {
    /// Keep polling
    Poll,
    /// Stop polling
    Stop,
}

pub(crate) trait PollBehavior<P> {
    /// The output type of the polled item
    type Output;

    /// Poll the underlying item and decides how the iteration should proceed
    ///
    /// # Return value
    /// The returned value coordinates two key aspects of the group iteration:
    /// - whether the group should keep iterating over the next items;
    ///    - `ControlFlow::Continue(_)` to inform that the group
    ///       should proceed to the next item
    ///    - `ControlFlow::Break(_)` to inform that the group
    ///       should stop iterating
    /// - whether the group should poll the same item again;
    ///    - [`PollAgain::Poll`] to inform the group to
    ///      mark that the item should be polled again
    ///    - [`PollAgain::Stop`] to inform the group to
    ///      stop polling this item
    fn poll(
        this: Pin<&mut P>,
        cx: &mut Context<'_>,
    ) -> ControlFlow<(Self::Output, PollAgain), PollAgain>;
}

pub(crate) struct PollFuture;

impl<F: Future> PollBehavior<F> for PollFuture {
    type Output = F::Output;

    fn poll(
        future: Pin<&mut F>,
        cx: &mut Context<'_>,
    ) -> ControlFlow<(Self::Output, PollAgain), PollAgain> {
        if let Poll::Ready(output) = future.poll(cx) {
            // return the futures output and inform the group to not poll it again
            ControlFlow::Break((output, PollAgain::Stop))
        } else {
            // future is not ready yet, keep polling
            ControlFlow::Continue(PollAgain::Poll)
        }
    }
}

pub(crate) struct PollStream;

impl<S: Stream> PollBehavior<S> for PollStream {
    type Output = S::Item;

    fn poll(
        stream: Pin<&mut S>,
        cx: &mut Context<'_>,
    ) -> ControlFlow<(Self::Output, PollAgain), PollAgain> {
        match stream.poll_next(cx) {
            // stop the iteration, keep polling this stream
            Poll::Ready(Some(item)) => ControlFlow::Break((item, PollAgain::Poll)),
            // continue the iteration, stop polling this stream
            Poll::Ready(None) => ControlFlow::Continue(PollAgain::Stop),
            // continue the iteration, continue polling this stream
            Poll::Pending => ControlFlow::Continue(PollAgain::Poll),
        }
    }
}
