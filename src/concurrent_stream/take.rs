use super::{ConcurrentStream, Consumer, ConsumerState};
use std::future::Future;

/// A concurrent iterator that only iterates over the first `n` iterations of `iter`.
///
/// This `struct` is created by the [`take`] method on [`ConcurrentStream`]. See its
/// documentation for more.
///
/// [`take`]: ConcurrentStream::take
/// [`ConcurrentStream`]: trait.ConcurrentStream.html
#[derive(Debug)]
pub struct Take<CS: ConcurrentStream> {
    inner: CS,
    limit: usize,
}

impl<CS: ConcurrentStream> Take<CS> {
    pub(crate) fn new(inner: CS, limit: usize) -> Self {
        Self { inner, limit }
    }
}

impl<CS: ConcurrentStream> ConcurrentStream for Take<CS> {
    type Item = CS::Item;
    type Future = CS::Future;

    async fn drive<C>(self, consumer: C) -> C::Output
    where
        C: Consumer<Self::Item, Self::Future>,
    {
        self.inner
            .drive(TakeConsumer {
                inner: consumer,
                count: 0,
                limit: self.limit,
            })
            .await
    }

    // NOTE: this is the only interesting bit in this module. When a limit is
    // set, this now starts using it.
    fn concurrency_limit(&self) -> Option<std::num::NonZeroUsize> {
        self.inner.concurrency_limit()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

struct TakeConsumer<C> {
    inner: C,
    count: usize,
    limit: usize,
}
impl<C, Item, Fut> Consumer<Item, Fut> for TakeConsumer<C>
where
    Fut: Future<Output = Item>,
    C: Consumer<Item, Fut>,
{
    type Output = C::Output;

    async fn send(&mut self, future: Fut) -> ConsumerState {
        self.count += 1;
        let state = self.inner.send(future).await;
        if self.count >= self.limit {
            ConsumerState::Break
        } else {
            state
        }
    }

    async fn progress(&mut self) -> ConsumerState {
        self.inner.progress().await
    }

    async fn finish(self) -> Self::Output {
        self.inner.finish().await
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;
    use futures_lite::stream;

    #[test]
    fn enumerate() {
        futures_lite::future::block_on(async {
            let mut n = 0;
            stream::iter(std::iter::from_fn(|| {
                let v = n;
                n += 1;
                Some(v)
            }))
            .co()
            .take(5)
            .for_each(|n| async move { assert!(dbg!(n) < 5) })
            .await;
        });
    }
}
