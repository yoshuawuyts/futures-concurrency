//! Concurrent execution of streams

mod drain;
mod enumerate;
mod for_each;
mod into_concurrent_iterator;
mod limit;
mod map;
mod take;
mod try_for_each;

use for_each::ForEachConsumer;
use std::future::Future;
use std::num::NonZeroUsize;
use try_for_each::TryForEachConsumer;

pub use enumerate::Enumerate;
pub use into_concurrent_iterator::{FromStream, IntoConcurrentStream};
pub use limit::Limit;
pub use map::Map;
pub use take::Take;

use self::drain::Drain;

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
    async fn send(&mut self, fut: Fut) -> ConsumerState;

    /// Make progress on the consumer while doing something else.
    ///
    /// It should always be possible to drop the future returned by this
    /// function. This is solely intended to keep work going on the `Consumer`
    /// while doing e.g. waiting for new futures from a stream.
    async fn progress(&mut self) -> ConsumerState;

    /// We have no more data left to send to the `Consumer`; wait for its
    /// output.
    async fn finish(self) -> Self::Output;
}

/// Concurrently operate over items in a stream
#[allow(async_fn_in_trait)]
pub trait ConcurrentStream {
    /// Which item will we be yielding?
    type Item;

    /// What's the type of the future containing our items?
    type Future: Future<Output = Self::Item>;

    /// Internal method used to define the behavior of this concurrent iterator.
    /// You should not need to call this directly. This method causes the
    /// iterator self to start producing items and to feed them to the consumer
    /// consumer one by one.
    async fn drive<C>(self, consumer: C) -> C::Output
    where
        C: Consumer<Self::Item, Self::Future>;

    /// How much concurrency should we apply?
    fn concurrency_limit(&self) -> Option<NonZeroUsize>;

    /// How many items could we potentially end up returning?
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    /// Creates a stream which gives the current iteration count as well as
    /// the next value.
    ///
    /// The value is determined by the moment the future is created, not the
    /// moment the future is evaluated.
    fn enumerate(self) -> Enumerate<Self>
    where
        Self: Sized,
    {
        Enumerate::new(self)
    }

    /// Iterate over each item in sequence
    async fn drain(self)
    where
        Self: Sized,
    {
        self.drive(Drain::new()).await
    }

    /// Obtain a simple pass-through adapter.
    fn limit(self, limit: Option<std::num::NonZeroUsize>) -> Limit<Self>
    where
        Self: Sized,
    {
        Limit::new(self, limit)
    }

    /// Creates a stream that yields the first `n`` elements, or fewer if the
    /// underlying iterator ends sooner.
    fn take(self, limit: usize) -> Take<Self>
    where
        Self: Sized,
    {
        Take::new(self, limit)
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

    /// Iterate over each item concurrently
    async fn for_each<F, Fut>(self, f: F)
    where
        Self: Sized,
        F: Fn(Self::Item) -> Fut,
        F: Clone,
        Fut: Future<Output = ()>,
    {
        let limit = self.concurrency_limit();
        self.drive(ForEachConsumer::new(limit, f)).await
    }

    /// Iterate over each item concurrently, short-circuit on error.
    ///
    /// If an error is returned this will cancel all other futures.
    async fn try_for_each<F, Fut, E>(self, f: F) -> Result<(), E>
    where
        Self: Sized,
        F: Fn(Self::Item) -> Fut,
        F: Clone,
        Fut: Future<Output = Result<(), E>>,
    {
        let limit = self.concurrency_limit();
        self.drive(TryForEachConsumer::new(limit, f)).await
    }
}

/// The state of the consumer, used to communicate back to the source.
#[derive(Debug)]
enum ConsumerState {
    /// The consumer is done making progress, and the `finish` method should be called.
    Break,
    /// The consumer is ready to keep making progress.
    Continue,
    /// The consumer currently holds no values and should not be called until
    /// more values have been provided to it.
    Empty,
}

#[cfg(test)]
mod test {
    use super::*;
    use futures_lite::prelude::*;
    use futures_lite::stream;

    #[test]
    fn drain() {
        futures_lite::future::block_on(async {
            stream::repeat(1)
                .take(5)
                .co()
                .map(|x| async move {
                    println!("{x:?}");
                })
                .drain()
                .await;
        });
    }

    #[test]
    fn for_each() {
        futures_lite::future::block_on(async {
            let s = stream::repeat(1).take(2);
            s.co()
                .limit(NonZeroUsize::new(3))
                .for_each(|x| async move {
                    println!("{x:?}");
                })
                .await;
        });
    }
}
