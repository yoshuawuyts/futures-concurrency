use futures_lite::StreamExt;

use crate::future::FutureGroup;

use super::Consumer;
use crate::prelude::*;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{ready, Context, Poll};

// OK: validated! - all bounds should check out
pub(crate) struct ForEachConsumer<F, Fut, T>
where
    F: Fn(T) -> Fut,
    Fut: Future<Output = ()>,
{
    is_done: bool,
    count: Arc<AtomicUsize>,
    group: Pin<Box<FutureGroup<InsertFut<Fut>>>>,
    limit: usize,
    f: F,
    _phantom: PhantomData<(Fut, T)>,
}

impl<F, Fut, T> ForEachConsumer<F, Fut, T>
where
    F: Fn(T) -> Fut,
    Fut: Future<Output = ()>,
{
    pub(crate) fn new(limit: usize, f: F) -> Self {
        Self {
            limit,
            f,
            _phantom: PhantomData,
            is_done: false,
            count: Arc::new(AtomicUsize::new(0)),
            group: Box::pin(FutureGroup::new()),
        }
    }
}

// OK: validated! - we push types `B` into the next consumer
impl<F, Fut, T> Consumer<T> for ForEachConsumer<F, Fut, T>
where
    F: Fn(T) -> Fut,
    Fut: Future<Output = ()>,
{
    type Output = ();

    async fn send<Fut2: Future<Output = T>>(&mut self, future: Fut2) {
        if self.is_done {
            self.group.next().await;
        }

        // ORDERING: this is single-threaded so `Relaxed` is ok
        match self.count.load(Ordering::Relaxed) {
            // 1. This is our base case: there are no items in the group, so we
            // first have to wait for an item to become available from the
            // stream.
            0 => {
                let item = future.await;
                // ORDERING: this is single-threaded so `Relaxed` is ok
                self.count.fetch_add(1, Ordering::Relaxed);
                let fut = insert_fut(&self.f, item, self.count.clone());
                self.group.as_mut().insert_pinned(fut);
            }

            // 2. Here our group still has capacity remaining, so we want to
            // keep pulling items from the stream while also processing items
            // currently in the group. If the group is done first, we do
            // nothing. If the stream has another item, we put it into the
            // group.
            n if n <= self.limit => {
                let a = async {
                    let item = future.await;
                    State::ItemReady(Some(item))
                };

                let b = async {
                    self.group.next().await;
                    State::GroupDone
                };
                match (a, b).race().await {
                    State::ItemReady(Some(item)) => {
                        // ORDERING: this is single-threaded so `Relaxed` is ok
                        self.count.fetch_add(1, Ordering::Relaxed);
                        let fut = insert_fut(&self.f, item, self.count.clone());
                        self.group.as_mut().insert_pinned(fut);
                    }
                    State::ItemReady(None) => {
                        self.is_done = true;
                    }
                    State::GroupDone => {} // do nothing, group just finished an item - we get to loop again
                }
            }

            // 3. Our group has no extra capacity, and so we don't pull any
            // additional items from the underlying stream. We have to wait for
            // items in the group to clear up first before we can pull more
            // items again.
            _ => {
                self.group.next().await;
            }
        }
    }

    async fn finish(mut self) -> Self::Output {
        // 4. We will no longer receive any additional futures from the
        // underlying stream; wait until all the futures in the group have
        // resolved.
        dbg!(&self.group);
        while let Some(_) = self.group.next().await {
            dbg!()
        }
    }
}

fn insert_fut<T, F, Fut>(f: F, item: T, count: Arc<AtomicUsize>) -> InsertFut<Fut>
where
    F: Fn(T) -> Fut,
    Fut: Future<Output = ()>,
{
    let fut = (f)(item);
    InsertFut { fut, count }
}

enum State<T> {
    ItemReady(Option<T>),
    GroupDone,
}

/// The future we're inserting into the stream
#[pin_project::pin_project]
struct InsertFut<Fut>
where
    Fut: Future<Output = ()>,
{
    #[pin]
    fut: Fut,
    count: Arc<AtomicUsize>,
}

impl<Fut: Future<Output = ()>> std::fmt::Debug for InsertFut<Fut> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InsertFut").finish()
    }
}

impl<'a, Fut> Future for InsertFut<Fut>
where
    Fut: Future<Output = ()>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        ready!(this.fut.poll(cx));
        // ORDERING: this is single-threaded so `Relaxed` is ok
        this.count.fetch_sub(1, Ordering::Relaxed);
        Poll::Ready(())
    }
}

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
            let limit = 1;
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
            let limit = 1;
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
