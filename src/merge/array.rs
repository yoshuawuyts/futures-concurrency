use crate::stream::IntoStream;
use crate::utils::{self, Fuse};
use crate::Merge as MergeTrait;

use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

impl<S, const N: usize> MergeTrait for [S; N]
where
    S: IntoStream,
{
    type Item = <Merge<S::IntoStream, N> as Stream>::Item;
    type IntoStream = Merge<S::IntoStream, N>;

    fn merge(self) -> Self::IntoStream {
        Merge::new(self.map(|i| i.into_stream()))
    }
}

/// A stream that merges multiple streams into a single stream.
///
/// This `struct` is created by the [`merge`] method on [`Stream`]. See its
/// documentation for more.
///
/// [`merge`]: trait.Stream.html#method.merge
/// [`Stream`]: trait.Stream.html
#[derive(Debug)]
#[pin_project::pin_project]
pub struct Merge<S, const N: usize>
where
    S: Stream,
{
    #[pin]
    streams: [Fuse<S>; N],
}

impl<S, const N: usize> Merge<S, N>
where
    S: Stream,
{
    pub(crate) fn new(streams: [S; N]) -> Self {
        Self {
            streams: streams.map(Fuse::new),
        }
    }
}

impl<S, const N: usize> Stream for Merge<S, N>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Randomize the indexes into our streams array. This ensures that when
        // multiple streams are ready at the same time, we don't accidentally
        // exhaust one stream before another.
        let mut arr: [usize; N] = core::array::from_fn(|n| n);
        arr.sort_by_cached_key(|_| utils::random(1000));

        // Iterate over our streams one-by-one. If a stream yields a value,
        // we exit early. By default we'll return `Poll::Ready(None)`, but
        // this changes if we encounter a `Poll::Pending`.
        let mut res = Poll::Ready(None);
        for index in arr {
            let stream = utils::get_pin_mut(this.streams.as_mut(), index).unwrap();
            match stream.poll_next(cx) {
                Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
                Poll::Ready(None) => continue,
                Poll::Pending => res = Poll::Pending,
            }
        }
        res
    }
}
