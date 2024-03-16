use futures_lite::{Stream, StreamExt};
use std::future::Ready;
use std::{future::ready, pin::pin};

use super::{ConcurrentStream, Consumer};

/// A concurrent for each implementation from a `Stream`
#[pin_project::pin_project]
#[derive(Debug)]
pub struct FromStream<S: Stream> {
    #[pin]
    iter: S,
}

impl<S> ConcurrentStream for FromStream<S>
where
    S: Stream,
{
    type Item = S::Item;
    type Future = Ready<Self::Item>;

    async fn drive<C>(self, mut consumer: C) -> C::Output
    where
        C: Consumer<Self::Item, Self::Future>,
    {
        let mut iter = pin!(self.iter);
        while let Some(item) = iter.next().await {
            consumer.send(ready(item)).await;
        }
        consumer.finish().await
    }
}

/// Convert into a concurrent stream
pub trait IntoConcurrentStream {
    /// The type of concurrent stream we're returning.
    type ConcurrentStream: ConcurrentStream;

    /// Convert `self` into a concurrent stream.
    fn co(self) -> Self::ConcurrentStream;
}

impl<S: Stream> IntoConcurrentStream for S {
    type ConcurrentStream = FromStream<S>;

    fn co(self) -> Self::ConcurrentStream {
        FromStream { iter: self }
    }
}
