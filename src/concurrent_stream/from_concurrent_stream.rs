use super::{ConcurrentStream, Consumer, ConsumerState, IntoConcurrentStream};
use crate::future::FutureGroup;
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use futures_lite::StreamExt;
use pin_project::pin_project;

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
        let stream = iter.into_co_stream();
        let mut output = Vec::with_capacity(stream.size_hint().1.unwrap_or_default());
        stream.drive(VecConsumer::new(&mut output)).await;
        output
    }
}

// TODO: replace this with a generalized `fold` operation
#[pin_project]
pub(crate) struct VecConsumer<'a, Fut: Future> {
    #[pin]
    group: FutureGroup<Fut>,
    output: &'a mut Vec<Fut::Output>,
}

impl<'a, Fut: Future> VecConsumer<'a, Fut> {
    pub(crate) fn new(output: &'a mut Vec<Fut::Output>) -> Self {
        Self {
            group: FutureGroup::new(),
            output,
        }
    }
}

impl<'a, Item, Fut> Consumer<Item, Fut> for VecConsumer<'a, Fut>
where
    Fut: Future<Output = Item>,
{
    type Output = ();

    async fn send(self: Pin<&mut Self>, future: Fut) -> super::ConsumerState {
        let mut this = self.project();
        // unbounded concurrency, so we just goooo
        this.group.as_mut().insert_pinned(future);
        ConsumerState::Continue
    }

    async fn progress(self: Pin<&mut Self>) -> super::ConsumerState {
        let mut this = self.project();
        while let Some(item) = this.group.next().await {
            this.output.push(item);
        }
        ConsumerState::Empty
    }
    async fn flush(self: Pin<&mut Self>) -> Self::Output {
        let mut this = self.project();
        while let Some(item) = this.group.next().await {
            this.output.push(item);
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
