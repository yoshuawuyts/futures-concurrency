use crate::future::FutureGroup;
use futures_lite::StreamExt;

use super::Consumer;
use std::future::Future;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::pin::{pin, Pin};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{ready, Context, Poll};

// OK: validated! - all bounds should check out
pub(crate) struct ForEachConsumer<A, T, F, B>
where
    A: Future<Output = T>,
    F: Fn(T) -> B,
    B: Future<Output = ()>,
{
    // NOTE: we can remove the `Arc` here if we're willing to make this struct self-referential
    count: Arc<AtomicUsize>,
    // TODO: remove the `Pin<Box>` from this signature by requiring this struct is pinned
    group: Pin<Box<FutureGroup<InsertFut<A, T, F, B>>>>,
    limit: usize,
    f: F,
    _phantom: PhantomData<(T, B)>,
}

impl<A, T, F, B> ForEachConsumer<A, T, F, B>
where
    A: Future<Output = T>,
    F: Fn(T) -> B,
    B: Future<Output = ()>,
{
    pub(crate) fn new(limit: NonZeroUsize, f: F) -> Self {
        Self {
            limit: limit.into(),
            f,
            _phantom: PhantomData,
            count: Arc::new(AtomicUsize::new(0)),
            group: Box::pin(FutureGroup::new()),
        }
    }
}

// OK: validated! - we push types `B` into the next consumer
impl<A, T, F, B> Consumer<T> for ForEachConsumer<A, T, F, B>
where
    A: Future<Output = T>,
    F: Fn(T) -> B,
    B: Future<Output = ()>,
{
    type Output = ();

    async fn send<A1: Future<Output = T>>(&mut self, future: A1) {
        // If we have no space, we're going to provide backpressure until we have space
        while self.count.load(Ordering::Relaxed) >= self.limit {
            self.group.next().await;
        }

        // Space was available! - insert the item for posterity
        self.count.fetch_add(1, Ordering::Relaxed);
        let fut = insert_fut(future, self.f, self.count.clone());
        self.group.as_mut().insert_pinned(fut);
    }

    async fn finish(mut self) -> Self::Output {
        // 4. We will no longer receive any additional futures from the
        // underlying stream; wait until all the futures in the group have
        // resolved.
        while let Some(_) = self.group.next().await {
            dbg!()
        }
    }
}

fn insert_fut<A, T, F, B>(input_fut: A, f: F, count: Arc<AtomicUsize>) -> InsertFut<A, T, F, B>
where
    A: Future<Output = T>,
    F: Fn(T) -> B,
    B: Future<Output = ()>,
{
    InsertFut {
        f,
        input_fut,
        count,
        _phantom: PhantomData,
    }
}

/// The future we're inserting into the stream
/// TODO: once we have TAITS we can remove this entire thing
#[pin_project::pin_project]
struct InsertFut<A, T, F, B>
where
    A: Future<Output = T>,
    F: Fn(T) -> B,
    B: Future<Output = ()>,
{
    f: F,
    #[pin]
    input_fut: A,
    count: Arc<AtomicUsize>,
    _phantom: PhantomData<T>,
}

impl<A, T, F, B> std::fmt::Debug for InsertFut<A, T, F, B>
where
    A: Future<Output = T>,
    F: Fn(T) -> B,
    B: Future<Output = ()>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InsertFut").finish()
    }
}

impl<A, T, F, B> Future for InsertFut<A, T, F, B>
where
    A: Future<Output = T>,
    F: Fn(T) -> B,
    B: Future<Output = ()>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        ready!(this.input_fut.poll(cx));
        // ORDERING: this is single-threaded so `Relaxed` is ok
        this.count.fetch_sub(1, Ordering::Relaxed);
        Poll::Ready(())
    }
}
type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[cfg(test)]
mod test {
    use super::*;
    use crate::concurrent_stream::{ConcurrentStream, IntoConcurrentStream};
    use futures_lite::stream;
    use std::sync::Arc;

    #[test]
    fn concurrency_one() {
        futures_lite::future::block_on(async {
            let count = Arc::new(AtomicUsize::new(0));
            let limit = NonZeroUsize::new(1).unwrap();
            stream::repeat(1)
                .take(2)
                .co()
                .for_each(limit, |n| {
                    let count = count.clone();
                    async move {
                        count.fetch_add(n, Ordering::Relaxed);
                    }
                })
                .await;
            assert_eq!(count.load(Ordering::Relaxed), 2);
        });
    }

    #[test]
    fn concurrency_three() {
        futures_lite::future::block_on(async {
            let count = Arc::new(AtomicUsize::new(0));
            let limit = NonZeroUsize::new(3).unwrap();
            stream::repeat(1)
                .take(10)
                .co()
                .for_each(limit, |n| {
                    let count = count.clone();
                    async move {
                        count.fetch_add(n, Ordering::Relaxed);
                    }
                })
                .await;
            assert_eq!(count.load(Ordering::Relaxed), 10);
        });
    }
}
