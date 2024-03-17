use super::{ConcurrentStream, Consumer, ConsumerState};
use crate::future::FutureGroup;
use core::future::Future;
use core::pin::Pin;
use futures_lite::StreamExt;

/// Conversion into a [`ConcurrentStream`]
pub trait IntoConcurrentStream {
    /// The type of the elements being iterated over.
    type Item;
    /// Which kind of iterator are we turning this into?
    type ConcurrentStream: ConcurrentStream<Item = Self::Item>;

    /// Convert `self` into a concurrent iterator.
    fn into_concurrent_stream(self) -> Self::ConcurrentStream;
}

impl<S: ConcurrentStream> IntoConcurrentStream for S {
    type Item = S::Item;
    type ConcurrentStream = S;

    fn into_concurrent_stream(self) -> Self::ConcurrentStream {
        self
    }
}

/// Conversion from a [`ConcurrentStream`]
#[allow(async_fn_in_trait)]
pub trait FromConcurrentStream<A>: Sized {
    /// Creates a value from a concurrent iterator.
    async fn from_concurrent_stream<T>(iter: T) -> Self
    where
        T: IntoConcurrentStream<Item = A>;
}

impl<T> FromConcurrentStream<T> for Vec<T> {
    async fn from_concurrent_stream<S>(iter: S) -> Self
    where
        S: IntoConcurrentStream<Item = T>,
    {
        let stream = iter.into_concurrent_stream();
        let mut output = Vec::with_capacity(stream.size_hint().1.unwrap_or_default());
        stream.drive(VecConsumer::new(&mut output)).await;
        output
    }
}

// TODO: replace this with a generalized `fold` operation
pub(crate) struct VecConsumer<'a, Fut: Future> {
    group: Pin<Box<FutureGroup<Fut>>>,
    output: &'a mut Vec<Fut::Output>,
}

impl<'a, Fut: Future> VecConsumer<'a, Fut> {
    pub(crate) fn new(output: &'a mut Vec<Fut::Output>) -> Self {
        Self {
            group: Box::pin(FutureGroup::new()),
            output,
        }
    }
}

impl<'a, Item, Fut> Consumer<Item, Fut> for VecConsumer<'a, Fut>
where
    Fut: Future<Output = Item>,
{
    type Output = ();

    async fn send(&mut self, future: Fut) -> super::ConsumerState {
        // unbounded concurrency, so we just goooo
        self.group.as_mut().insert_pinned(future);
        ConsumerState::Continue
    }

    async fn progress(&mut self) -> super::ConsumerState {
        while let Some(item) = self.group.next().await {
            self.output.push(item);
        }
        ConsumerState::Empty
    }
    async fn finish(mut self) -> Self::Output {
        while let Some(item) = self.group.next().await {
            self.output.push(item);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;
    use futures_lite::stream;

    #[test]
    fn collect() {
        futures_lite::future::block_on(async {
            let v: Vec<_> = stream::repeat(1).co().take(5).collect().await;
            assert_eq!(v, &[1, 1, 1, 1, 1]);
        });
    }
}
