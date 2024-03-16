use super::{ConcurrentStream, Consumer};
use std::future::Future;

#[derive(Debug)]
pub struct Passthrough<CS: ConcurrentStream> {
    inner: CS,
}

impl<CS: ConcurrentStream> Passthrough<CS> {
    pub(crate) fn new(inner: CS) -> Self {
        Self { inner }
    }
}

impl<CS: ConcurrentStream> ConcurrentStream for Passthrough<CS> {
    type Item = CS::Item;
    type Future = CS::Future;

    async fn drive<C>(self, consumer: C) -> C::Output
    where
        C: Consumer<Self::Item, Self::Future>,
    {
        self.inner
            .drive(PassthroughConsumer { inner: consumer })
            .await
    }

    fn concurrency_limit(&self) -> Option<std::num::NonZeroUsize> {
        self.inner.concurrency_limit()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

struct PassthroughConsumer<C> {
    inner: C,
}
impl<C, Item, Fut> Consumer<Item, Fut> for PassthroughConsumer<C>
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
