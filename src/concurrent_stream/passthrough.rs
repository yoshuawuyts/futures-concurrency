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

    async fn drive<C, Fut>(self, consumer: C) -> C::Output
    where
        Fut: Future<Output = Self::Item>,
        C: Consumer<Self::Item, Fut>,
    {
        self.inner
            .drive(PassthroughConsumer { inner: consumer })
            .await
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

    async fn send(&mut self, future: Fut) {
        self.inner.send(future).await;
    }

    async fn progress(&mut self) {
        self.inner.progress().await;
    }

    async fn finish(self) -> Self::Output {
        self.inner.finish().await
    }
}
