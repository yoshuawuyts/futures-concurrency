use core::{
    marker::PhantomData,
    ops::ControlFlow,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use alloc::{collections::BTreeSet, fmt};
use slab::Slab;
use smallvec::{smallvec, SmallVec};

use crate::utils::{PollVec, WakerVec};

const GROUP_GROWTH_FACTOR: usize = 2;

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
            self.resize((self.cap + 1) * GROUP_GROWTH_FACTOR);
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
        // todo: less unsafe

        if !self.has_capacity() {
            let r = unsafe { &mut self.as_mut().get_unchecked_mut() };
            r.resize((r.cap + 1) * GROUP_GROWTH_FACTOR);
        }

        let mut this = self.project();
        let items = unsafe { &mut this.items.as_mut().get_unchecked_mut() };
        let index = items.insert(item);
        this.keys.insert(index);

        // set the corresponding state
        this.states[index].set_pending();
        this.wakers.readiness().set_ready(index);

        *this.len += 1;
        Key(index)
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

    // todo: rename to reserve
    pub fn resize(&mut self, cap: usize) {
        if self.len + cap < self.cap {
            return;
        }
        self.wakers.resize(cap);
        self.states.resize(cap);
        self.items.reserve_exact(cap);
        self.cap = cap;
    }

    pub fn any_ready(&self) -> bool {
        self.wakers.readiness().any_ready()
    }

    pub fn set_top_waker(&mut self, waker: &Waker) {
        self.wakers.readiness().set_waker(waker);
    }

    pub fn can_progress_index(&self, index: usize) -> bool {
        self.states[index].is_pending() && self.wakers.readiness().clear_ready(index)
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

// ----
// prototyping

use theory::*;

impl<A, B> InnerGroup<A, B>
where
    B: PollBehavior<Polling = A>,
{
    pub fn poll_next_inner(
        mut self: Pin<&mut Self>,
        cx: &Context<'_>,
    ) -> Poll<Option<(Key, B::Poll)>> {
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        // short-circuit if we have no items to iterate over
        if this.is_empty() {
            return Poll::Ready(None);
        }

        // set the top-level waker and check readiness
        this.set_top_waker(cx.waker());
        if !this.any_ready() {
            // nothing is ready yet
            return Poll::Pending;
        }

        let mut done_count = 0;
        let group_len = this.len();
        let mut removal_queue: SmallVec<[_; 10]> = smallvec![];

        let mut ret = Poll::Pending;

        for index in this.keys.iter().cloned() {
            if !this.can_progress_index(index) {
                continue;
            }

            // obtain the intermediate waker
            let mut cx = Context::from_waker(this.wakers.get(index).unwrap());

            let pollable = unsafe { Pin::new_unchecked(&mut this.items[index]) };
            match B::poll(pollable, &mut cx) {
                Poll::Ready(ControlFlow::Break((result, PollAgain::Stop))) => {
                    for item in removal_queue {
                        this.remove(Key(item));
                    }
                    this.remove(Key(index));
                    return Poll::Ready(Some((Key(index), result)));
                }
                Poll::Ready(ControlFlow::Break((result, PollAgain::Poll))) => {
                    this.states[index].set_pending();
                    this.wakers.readiness().set_ready(index);

                    ret = Poll::Ready(Some((Key(index), result)));
                    break;
                }
                Poll::Ready(ControlFlow::Continue(_)) => {
                    done_count += 1;
                    removal_queue.push(index);
                    continue;
                }
                Poll::Pending => continue,
            }
        }
        for item in removal_queue {
            this.remove(Key(item));
        }

        if done_count == group_len {
            return Poll::Ready(None);
        }

        ret
    }
}

pub mod theory {
    use core::future::Future;
    use core::marker::PhantomData;
    use core::ops::ControlFlow;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    use futures_core::Stream;

    pub enum PollAgain {
        Poll,
        Stop,
    }

    pub trait PollBehavior {
        type Item;
        type Polling;
        type Poll;

        fn poll(
            this: Pin<&mut Self::Polling>,
            cx: &mut Context<'_>,
        ) -> Poll<ControlFlow<(Self::Poll, PollAgain)>>;
    }

    #[derive(Debug)]
    pub struct PollFuture<F>(PhantomData<F>);

    impl<F: Future> PollBehavior for PollFuture<F> {
        type Item = F::Output;
        type Polling = F;
        type Poll = Self::Item;

        fn poll(
            this: Pin<&mut Self::Polling>,
            cx: &mut Context<'_>,
        ) -> Poll<ControlFlow<(Self::Poll, PollAgain)>> {
            if let Poll::Ready(item) = this.poll(cx) {
                Poll::Ready(ControlFlow::Break((item, PollAgain::Stop)))
            } else {
                Poll::Pending
            }
        }
    }

    #[derive(Debug)]
    pub struct PollStream<S>(PhantomData<S>);

    impl<S: Stream> PollBehavior for PollStream<S> {
        type Item = S::Item;
        type Polling = S;
        type Poll = Self::Item;

        fn poll(
            this: Pin<&mut Self::Polling>,
            cx: &mut Context<'_>,
        ) -> Poll<ControlFlow<(Self::Poll, PollAgain)>> {
            match this.poll_next(cx) {
                Poll::Ready(Some(item)) => Poll::Ready(ControlFlow::Break((item, PollAgain::Poll))),
                Poll::Ready(None) => Poll::Ready(ControlFlow::Continue(())),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}
