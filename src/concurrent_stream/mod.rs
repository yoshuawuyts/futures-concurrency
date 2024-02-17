//! Concurrent execution of streams

use crate::future::{FutureGroup, Race};
use futures_lite::{Stream, StreamExt};
use std::clone::Clone;
use std::future::Future;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Concurrently map the items coming out of a sequential stream, using `limit`
/// as the max concurrency.
///
/// This implementation does not suffer from the "concurrent iterator" issue,
/// because it will always make forward progress.
pub async fn concurrent_for_each<I, F, Fut>(mut stream: I, f: F, limit: usize)
where
    I: Stream + Unpin,
    F: Fn(I::Item) -> Fut,
    Fut: Future<Output = ()>,
{
    let mut is_done = false;
    let count = Arc::new(AtomicUsize::new(0));
    let mut group = pin!(FutureGroup::new());

    loop {
        if is_done {
            group.next().await;
        }

        // ORDERING: this is single-threaded so `Relaxed` is ok
        match count.load(Ordering::Relaxed) {
            // 1. This is our base case: there are no items in the group, so we
            // first have to wait for an item to become available from the
            // stream.
            0 => match stream.next().await {
                Some(item) => {
                    // ORDERING: this is single-threaded so `Relaxed` is ok
                    count.fetch_add(1, Ordering::Relaxed);
                    let fut = insert_fut(&f, item, count.clone());
                    group.as_mut().insert_pinned(fut);
                }
                None => {
                    return;
                }
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
                        // ORDERING: this is single-threaded so `Relaxed` is ok
                        count.fetch_add(1, Ordering::Relaxed);
                        let fut = insert_fut(&f, item, count.clone());
                        group.as_mut().insert_pinned(fut);
                    }
                    State::ItemReady(None) => {
                        is_done = true;
                    }
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

async fn insert_fut<T, F, Fut>(f: F, item: T, count: Arc<AtomicUsize>)
where
    F: Fn(T) -> Fut,
    Fut: Future<Output = ()>,
{
    (f)(item).await;
    // ORDERING: this is single-threaded so `Relaxed` is ok
    count.fetch_sub(1, Ordering::Relaxed);
}

enum State<T> {
    ItemReady(Option<T>),
    GroupDone,
}

#[cfg(test)]
mod test {
    use super::*;
    use futures_lite::stream;

    #[test]
    fn concurrency_one() {
        futures_lite::future::block_on(async {
            let count = Arc::new(AtomicUsize::new(0));
            let s = stream::repeat(1).take(2);
            let limit = 1;
            let map = |n| {
                let count = count.clone();
                async move {
                    count.fetch_add(n, Ordering::Relaxed);
                }
            };
            concurrent_for_each(s, map, limit).await;
            assert_eq!(count.load(Ordering::Relaxed), 2);
        });
    }

    #[test]
    fn concurrency_three() {
        futures_lite::future::block_on(async {
            let count = Arc::new(AtomicUsize::new(0));
            let s = stream::repeat(1).take(10);
            let limit = 3;
            let map = |n| {
                let count = count.clone();
                async move {
                    count.fetch_add(n, Ordering::Relaxed);
                }
            };
            concurrent_for_each(s, map, limit).await;
            assert_eq!(count.load(Ordering::Relaxed), 10);
        });
    }
}
