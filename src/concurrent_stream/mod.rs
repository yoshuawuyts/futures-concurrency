//! Concurrent execution of streams

use crate::future::{FutureGroup, Race};
use futures_lite::{Stream, StreamExt};
use std::clone::Clone;
use std::future::Future;
use std::pin::{pin, Pin};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

/// Concurrently map the items coming out of a sequential stream, using `limit`
/// as the max concurrency.
///
/// This implementation does not suffer from the "concurrent iterator" issue,
/// because it will always make forward progress.
pub async fn concurrent_for_each<I, F>(stream: I, f: F, limit: usize)
where
    I: Stream + Unpin,
    F: Fn(I::Item),
{
    let mut stream = stream.fuse();
    let count = Arc::new(AtomicUsize::new(0));
    let mut group = FutureGroup::new();

    loop {
        match count.load(Ordering::Relaxed) {
            // 1. This is our base case: there are no items in the group, so we
            // first have to wait for an item to become available from the
            // stream.
            0 => match stream.next().await {
                Some(item) => {
                    count.fetch_add(1, Ordering::Relaxed);
                    let fut = CustomFut::new(&f, item, count.clone());
                    group.insert(fut);
                }
                None => return,
            },

            // 2. Here our group still has capacity remaining, so we want to
            // keep pulling items from the stream while also processing items
            // currently in the group. If the group is done first, we do
            // nothing. If the stream has another item, we put it into the
            // group.
            n if n <= limit => {
                let a = async {
                    let item = stream.next().await;
                    State::ItemReady(item)
                };

                let b = async {
                    group.next().await;
                    State::GroupDone
                };
                match (a, b).race().await {
                    State::ItemReady(Some(item)) => {
                        count.fetch_add(1, Ordering::Relaxed);
                        let fut = CustomFut::new(&f, item, count.clone());
                        group.insert(fut);
                    }
                    State::ItemReady(None) => {} // do nothing, stream is done
                    State::GroupDone => {} // do nothing, group just finished an item - we get to loop again
                }
            }

            // 3. Our group has no extra capacity, and so we don't pull any
            // additional items from the underlying stream. We have to wait for
            // items in the group to clear up first before we can pull more
            // items again.
            _ => {
                group.next().await;
            }
        }
    }
}

enum State<T> {
    ItemReady(Option<T>),
    GroupDone,
}

/// This is a custom future implementation to ensure that our internal
/// `FutureGroup` impl `: Unpin`. We need to give it a concrete type, with a
/// concrete `Map` impl, and guarantee it implements `Unpin`.
#[pin_project::pin_project]
struct CustomFut<F, T> {
    f: F,
    item: Option<T>,
    count: Arc<AtomicUsize>, // lmao, don't judge me
}

impl<F, T> CustomFut<F, T> {
    fn new(f: F, item: T, count: Arc<AtomicUsize>) -> Self {
        Self {
            f,
            item: Some(item),
            count,
        }
    }
}

impl<F, T> Future for CustomFut<F, T>
where
    F: Fn(T),
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        (this.f)(this.item.take().unwrap());
        this.count.fetch_sub(1, Ordering::Relaxed);
        Poll::Ready(())
    }
}
