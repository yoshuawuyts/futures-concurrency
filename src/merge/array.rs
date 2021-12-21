use crate::utils::{random, Fuse};
use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

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
        let this = self.project();

        // Randomize our streams array. This ensures that when multiple streams
        // are ready at the same time, we don't accidentally exhaust one stream
        // before another.
        this.streams.sort_by_cached_key(|_| random(1000));

        // Iterate over our streams one-by-one. If a stream yields a value,
        // we exit early. By default we'll return `Poll::Ready(None)`, but
        // this changes if we encounter a `Poll::Pending`.
        let mut res = Poll::Ready(None);
        for stream in this.streams.iter_mut() {
            // SAFETY: this is safe because `Self` never moves and doesn't have any `Drop` impls.
            match unsafe { Pin::new_unchecked(stream) }.poll_next(cx) {
                Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
                Poll::Ready(None) => continue,
                Poll::Pending => res = Poll::Pending,
            }
        }
        res
    }
}
