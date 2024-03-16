//! Concurrent execution of streams

mod concurrent_foreach;
mod drain;
mod into_concurrent_iterator;
// mod map;
mod passthrough;

pub use into_concurrent_iterator::{FromStream, IntoConcurrentStream};
// pub use map::Map;
use passthrough::Passthrough;
use std::future::Future;

#[allow(missing_docs)]
#[allow(async_fn_in_trait)]
pub trait Consumer<Item> {
    type Output;

    /// Consume a single item at a time.
    async fn send<Fut: Future<Output = Item>>(&mut self, fut: Fut);

    /// The `Consumer` is done; we're ready to return the output.
    async fn finish(self) -> Self::Output;
}

/// Concurrently operate over items in a stream
#[allow(missing_docs)]
#[allow(async_fn_in_trait)]
pub trait ConcurrentStream {
    type Item;

    async fn drive<C>(self, consumer: C) -> C::Output
    where
        C: Consumer<Self::Item>;

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

    // /// Iterate over each item concurrently
    // async fn for_each<F, Fut>(self, limit: usize, f: F)
    // where
    //     Self: Sized,

    //     F: Fn(Self::Item) -> Fut,
    //     Fut: Future<Output = ()>,
    // {
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
            s.co().drain().await;
        });
    }
}
