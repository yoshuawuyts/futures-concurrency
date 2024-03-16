use super::{ConcurrentStream, Consumer};
use std::{future::Future, num::NonZeroUsize};

#[derive(Debug)]
pub struct Limit<CS: ConcurrentStream> {
    inner: CS,
    limit: Option<NonZeroUsize>,
}

impl<CS: ConcurrentStream> Limit<CS> {
    pub(crate) fn new(inner: CS, limit: Option<NonZeroUsize>) -> Self {
        Self { inner, limit }
    }
}

impl<CS: ConcurrentStream> ConcurrentStream for Limit<CS> {
    type Item = CS::Item;
    type Future = CS::Future;

    async fn drive<C>(self, consumer: C) -> C::Output
    where
        C: Consumer<Self::Item, Self::Future>,
    {
        self.inner.drive(LimitConsumer { inner: consumer }).await
    }

    // NOTE: this is the only interesting bit in this module. When a limit is
    // set, this now starts using it.
    fn concurrency_limit(&self) -> Option<std::num::NonZeroUsize> {
        self.limit
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

struct LimitConsumer<C> {
    inner: C,
}
impl<C, Item, Fut> Consumer<Item, Fut> for LimitConsumer<C>
where
    Fut: Future<Output = Item>,
    C: Consumer<Item, Fut>,
{
    type Output = C::Output;

    async fn send(&mut self, future: Fut) -> super::ConsumerState {
        self.inner.send(future).await
    }

    async fn progress(&mut self) -> super::ConsumerState {
        self.inner.progress().await
    }

    async fn finish(self) -> Self::Output {
        self.inner.finish().await
    }
}
