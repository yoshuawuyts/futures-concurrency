use super::Merge as MergeTrait;
use crate::stream::IntoStream;
use crate::utils::{self, Fuse};

use core::fmt;
use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A stream that merges multiple streams into a single stream.
///
/// This `struct` is created by the [`merge`] method on the [`Merge`] trait. See its
/// documentation for more.
///
/// [`merge`]: trait.Merge.html#method.merge
/// [`Merge`]: trait.Merge.html
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

impl<S, const N: usize> fmt::Debug for Merge<S, N>
where
    S: Stream + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.streams.iter()).finish()
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
        let mut arr: [usize; N] = {
            // this is an inlined version of `core::array::from_fn`
            // TODO: replace this with `core::array::from_fn` when it becomes stable
            let cb = |n| n;
            let mut idx = 0;
            [(); N].map(|_| {
                let res = cb(idx);
                idx += 1;
                res
            })
        };
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

impl<S, const N: usize> MergeTrait for [S; N]
where
    S: IntoStream,
{
    type Item = <Merge<S::IntoStream, N> as Stream>::Item;
    type Stream = Merge<S::IntoStream, N>;

    fn merge(self) -> Self::Stream {
        Merge::new(self.map(|i| i.into_stream()))
    }
}
