use core::{pin::Pin, task::Waker};

use alloc::{collections::BTreeSet, fmt};
use slab::Slab;

use crate::utils::{PollVec, WakerVec};

const GROUP_GROWTH_FACTOR: usize = 2;

#[pin_project::pin_project]
pub struct GroupInner<A> {
    #[pin]
    pub items: Slab<A>,
    pub wakers: WakerVec,
    pub states: PollVec,
    pub keys: BTreeSet<usize>,
    cap: usize,
    len: usize,
}

impl<A> GroupInner<A> {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            items: Slab::with_capacity(cap),
            wakers: WakerVec::new(cap),
            states: PollVec::new(cap),
            keys: BTreeSet::new(),
            cap,
            len: 0,
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
}

/// Keyed operations
impl<A> GroupInner<A> {
    // move to other impl block
    pub fn contains_key(&self, key: Key) -> bool {
        self.items.contains(key.0)
    }
}

impl<A> Default for GroupInner<A> {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

impl<A> fmt::Debug for GroupInner<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GroupInner")
            .field("cap", &self.cap)
            .field("len", &self.len)
            .finish()
    }
}

/// A key used to index into the `FutureGroup` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(pub usize);
