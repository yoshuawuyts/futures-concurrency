//! Concurrent execution of streams

mod drain;
// mod for_each;
mod into_concurrent_iterator;
mod map;
mod passthrough;

// use for_each::ForEachConsumer;
use passthrough::Passthrough;
use std::future::Future;
// use std::num::NonZeroUsize;

pub use into_concurrent_iterator::{FromStream, IntoConcurrentStream};
pub use map::Map;

/// Describes a type which can receive data.
///
/// # Type Generics
/// - `Item` in this context means the item that it will  repeatedly receive.
/// - `Future` in this context refers to the future type repeatedly submitted to it.
#[allow(async_fn_in_trait)]
pub trait Consumer<Item, Fut>
where
    Fut: Future<Output = Item>,
{
    /// What is the type of the item we're returning when completed?
    type Output;

    /// Send an item down to the next step in the processing queue.
    async fn send(&mut self, fut: Fut);

    /// Make progress on the consumer while doing something else.
    ///
    /// It should always be possible to drop the future returned by this
    /// function. This is solely intended to keep work going on the `Consumer`
    /// while doing e.g. waiting for new futures from a stream.
    async fn progress(&mut self);

    /// We have no more data left to send to the `Consumer`; wait for its
    /// output.
    async fn finish(self) -> Self::Output;
}

/// Concurrently operate over items in a stream
#[allow(missing_docs)]
#[allow(async_fn_in_trait)]
pub trait ConcurrentStream {
    type Item;

    type Future: Future<Output = Self::Item>;

    /// Internal method used to define the behavior of this concurrent iterator.
    /// You should not need to call this directly. This method causes the
    /// iterator self to start producing items and to feed them to the consumer
    /// consumer one by one.
    async fn drive<C>(self, consumer: C) -> C::Output
    where
        C: Consumer<Self::Item, Self::Future>;

    fn passthrough(self) -> Passthrough<Self>
    where
        Self: Sized,
    {
        Passthrough::new(self)
    }

    /// Iterate over each item in sequence
    async fn drain(self)
    where
        Self: Sized,
    {
        self.drive(drain::Drain {}).await
    }

    /// Convert items from one type into another
    fn map<F, FutB, B>(self, f: F) -> Map<Self, F, Self::Future, Self::Item, FutB, B>
    where
        Self: Sized,
        F: Fn(Self::Item) -> FutB,
        F: Clone,
        FutB: Future<Output = B>,
    {
        Map::new(self, f)
    }

    // /// Iterate over each item concurrently
    // async fn for_each<F, Fut>(self, limit: NonZeroUsize, f: F)
    // where
    //     Self: Sized,

    //     F: Fn(Self::Item) -> Fut,
    //     Fut: Future<Output = ()>,
    // {
    //     self.drive(ForEachConsumer::new(limit, f)).await
    // }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures_lite::prelude::*;
    use futures_lite::stream;

    #[test]
    fn drain() {
        futures_lite::future::block_on(async {
            let s = stream::repeat(1).take(2);
            s.co().map(|x| async move { dbg!(x) }).drain().await;
        });
    }

    //     #[test]
    //     fn for_each() {
    //         futures_lite::future::block_on(async {
    //             let s = stream::repeat(1).take(2);
    //             let limit = NonZeroUsize::new(3).unwrap();
    //             s.co()
    //                 .for_each(limit, |x| async move {
    //                     dbg!(x);
    //                 })
    //                 .await;
    //         });
    //     }
}
