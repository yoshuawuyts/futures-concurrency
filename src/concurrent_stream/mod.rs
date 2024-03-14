//! Concurrent execution of streams

use crate::future::{FutureGroup, Race};
use core::future::Future;
use core::pin::pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use futures_lite::{Stream, StreamExt};

mod map;

pub use map::Map;

#[allow(missing_docs)]
pub trait ConcurrentStream {
    type Item;
    type Future<'a>: Future<Output = Option<Self::Item>>
    where
        Self: 'a;
    fn next(&self) -> Self::Future<'_>;

    /// Map this stream's output to a different type.
    fn map<F, Fut, B>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
        F: Fn(Self::Item) -> Fut,
        Fut: Future<Output = B>,
    {
        Map::new(self, f)
    }

    fn for_each<F, Fut>(self, f: F, limit: usize) -> impl Future<Output = ()>
    where
        Self: Sized,
        F: Fn(Self::Item) -> Fut,
        Fut: Future<Output = ()>,
    {
        async move {
            let mut is_done = false;
            // let count = Arc::new(AtomicUsize::new(0));
            let count = AtomicUsize::new(0);
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
                    0 => match self.next().await {
                        Some(item) => {
                            // ORDERING: this is single-threaded so `Relaxed` is ok
                            count.fetch_add(1, Ordering::Relaxed);
                            let fut = insert_fut(&f, item, &count);
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
                            let item = self.next().await;
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
                                let fut = insert_fut(&f, item, &count);
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
    }
}

async fn insert_fut<T, F, Fut>(f: F, item: T, count: &AtomicUsize)
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

// #[cfg(test)]
// mod test {
//     use super::*;
//     use futures_lite::stream;

//     #[test]
//     fn concurrency_one() {
//         futures_lite::future::block_on(async {
//             let count = Arc::new(AtomicUsize::new(0));
//             let s = stream::repeat(1).take(2);
//             let limit = 1;
//             let map = |n| {
//                 let count = count.clone();
//                 async move {
//                     count.fetch_add(n, Ordering::Relaxed);
//                 }
//             };
//             concurrent_for_each(s, map, limit).await;
//             assert_eq!(count.load(Ordering::Relaxed), 2);
//         });
//     }

//     #[test]
//     fn concurrency_three() {
//         futures_lite::future::block_on(async {
//             let count = Arc::new(AtomicUsize::new(0));
//             let s = stream::repeat(1).take(10);
//             let limit = 3;
//             let map = |n| {
//                 let count = count.clone();
//                 async move {
//                     count.fetch_add(n, Ordering::Relaxed);
//                 }
//             };
//             concurrent_for_each(s, map, limit).await;
//             assert_eq!(count.load(Ordering::Relaxed), 10);
//         });
//     }
// }
