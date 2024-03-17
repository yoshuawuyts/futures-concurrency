use super::{Consumer, ConsumerState};
use crate::future::FutureGroup;

use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use futures_lite::StreamExt;

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

    async fn send(&mut self, future: Fut) -> super::ConsumerState {
        // unbounded concurrency, so we just goooo
        self.group.as_mut().insert_pinned(future);
        ConsumerState::Continue
    }

    async fn progress(&mut self) -> super::ConsumerState {
        while let Some(_) = self.group.next().await {}
        ConsumerState::Empty
    }
    async fn finish(mut self) -> Self::Output {
        while let Some(_) = self.group.next().await {}
    }
}
