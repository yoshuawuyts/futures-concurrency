use super::{ConcurrentStream, Consumer};
use std::{
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll},
};

/// A concurrent iterator that yields the current count and the element during iteration.
///
/// This `struct` is created by the [`enumerate`] method on [`ConcurrentStream`]. See its
/// documentation for more.
///
/// [`enumerate`]: ConcurrentStream::enumerate
/// [`ConcurrentStream`]: trait.ConcurrentStream.html
#[derive(Debug)]
pub struct Enumerate<CS: ConcurrentStream> {
    inner: CS,
}

impl<CS: ConcurrentStream> Enumerate<CS> {
    pub(crate) fn new(inner: CS) -> Self {
        Self { inner }
    }
}

impl<CS: ConcurrentStream> ConcurrentStream for Enumerate<CS> {
    type Item = (usize, CS::Item);
    type Future = EnumerateFuture<CS::Future, CS::Item>;

    async fn drive<C>(self, consumer: C) -> C::Output
    where
        C: Consumer<Self::Item, Self::Future>,
    {
        self.inner
            .drive(EnumerateConsumer {
                inner: consumer,
                count: 0,
            })
            .await
    }

    fn concurrency_limit(&self) -> Option<std::num::NonZeroUsize> {
        self.inner.concurrency_limit()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

struct EnumerateConsumer<C> {
    inner: C,
    count: usize,
}
impl<C, Item, Fut> Consumer<Item, Fut> for EnumerateConsumer<C>
where
    Fut: Future<Output = Item>,
    C: Consumer<(usize, Item), EnumerateFuture<Fut, Item>>,
{
    type Output = C::Output;

    async fn send(&mut self, future: Fut) -> super::ConsumerState {
        let count = self.count;
        self.count += 1;
        self.inner.send(EnumerateFuture::new(future, count)).await
    }

    async fn progress(&mut self) -> super::ConsumerState {
        self.inner.progress().await
    }

    async fn finish(self) -> Self::Output {
        self.inner.finish().await
    }
}

/// Takes a future and maps it to another future via a closure
#[derive(Debug)]
#[pin_project::pin_project]
pub struct EnumerateFuture<FutT, T>
where
    FutT: Future<Output = T>,
{
    done: bool,
    #[pin]
    fut_t: FutT,
    count: usize,
}

impl<FutT, T> EnumerateFuture<FutT, T>
where
    FutT: Future<Output = T>,
{
    fn new(fut_t: FutT, count: usize) -> Self {
        Self {
            done: false,
            fut_t,
            count,
        }
    }
}

impl<FutT, T> Future for EnumerateFuture<FutT, T>
where
    FutT: Future<Output = T>,
{
    type Output = (usize, T);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.done {
            panic!("future has already been polled to completion once");
        }

        let item = ready!(this.fut_t.poll(cx));
        *this.done = true;
        Poll::Ready((*this.count, item))
    }
}

#[cfg(test)]
mod test {
    use crate::concurrent_stream::{ConcurrentStream, IntoConcurrentStream};
    use futures_lite::stream;
    use futures_lite::StreamExt;
    use std::num::NonZeroUsize;

    #[test]
    fn enumerate() {
        futures_lite::future::block_on(async {
            let mut n = 0;
            stream::iter(std::iter::from_fn(|| {
                let v = n;
                n += 1;
                Some(v)
            }))
            .take(5)
            .co()
            .limit(NonZeroUsize::new(1))
            .enumerate()
            .for_each(|(index, n)| async move {
                assert_eq!(index, n);
            })
            .await;
        });
    }
}
