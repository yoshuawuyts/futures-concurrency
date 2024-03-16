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

    async fn drive<C>(self, consumer: C) -> C::Output
    where
        C: Consumer<Self::Item>,
    {
        self.inner
            .drive(PassthroughConsumer { inner: consumer })
            .await
    }
}

struct PassthroughConsumer<C> {
    inner: C,
}
impl<C, Item> Consumer<Item> for PassthroughConsumer<C>
where
    C: Consumer<Item>,
{
    type Output = C::Output;

    async fn send<Fut: Future<Output = Item>>(&mut self, future: Fut) {
        self.inner.send(future).await;
    }

    async fn finish(self) -> Self::Output {
        self.inner.finish().await
    }
}
