use crate::future::FutureGroup;
use futures_lite::StreamExt;

use super::Consumer;
use core::future::Future;
use std::pin::Pin;

pub(crate) struct Drain<Fut: Future> {
    group: Pin<Box<FutureGroup<Fut>>>,
}

impl<Fut: Future> Drain<Fut> {
    pub(crate) fn new() -> Self {
        Self {
            group: Box::pin(FutureGroup::new()),
        }
    }
}

impl<Item, Fut> Consumer<Item, Fut> for Drain<Fut>
where
    Fut: Future<Output = Item>,
{
    type Output = ();

    async fn send(&mut self, future: Fut) {
        // unbounded concurrency, so we just goooo
        self.group.as_mut().insert_pinned(future);
    }

    async fn progress(&mut self) {
        while let Some(_) = self.group.next().await {}
    }
    async fn finish(mut self) -> Self::Output {
        while let Some(_) = self.group.next().await {}
    }
}
