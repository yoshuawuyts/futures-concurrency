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
pub struct Merge<S>
where
    S: Stream,
{
    #[pin]
    streams: Vec<Fuse<S>>,
}

impl<S> Merge<S>
where
    S: Stream,
{
    pub(crate) fn new(streams: Vec<S>) -> Self {
        Self {
            streams: streams.into_iter().map(Fuse::new).collect(),
        }
    }
}

impl<S> fmt::Debug for Merge<S>
where
    S: Stream + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.streams.iter()).finish()
    }
}

impl<S> Stream for Merge<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Randomize the indexes into our streams array. This ensures that when
        // multiple streams are ready at the same time, we don't accidentally
        // exhaust one stream before another.
        // Randomize the indexes into our streams array. This ensures that when
        // multiple streams are ready at the same time, we don't accidentally
        // exhaust one stream before another.
        let indexes: Vec<_> = (0..this.streams.len()).collect();
        // indexes.sort_by_cached_key(|_| utils::random(1000));

        // Iterate over our streams one-by-one. If a stream yields a value,
        // we exit early. By default we'll return `Poll::Ready(None)`, but
        // this changes if we encounter a `Poll::Pending`.
        let mut res = Poll::Ready(None);
        for index in indexes {
            let stream = utils::get_pin_mut_from_vec(this.streams.as_mut(), index).unwrap();
            match stream.poll_next(cx) {
                Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
                Poll::Ready(None) => continue,
                Poll::Pending => res = Poll::Pending,
            }
        }
        res
    }
}

impl<S> MergeTrait for Vec<S>
where
    S: IntoStream,
{
    type Item = <Merge<S::IntoStream> as Stream>::Item;
    type Stream = Merge<S::IntoStream>;

    fn merge(self) -> Self::Stream {
        Merge::new(self.into_iter().map(|i| i.into_stream()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_lite::future::block_on;
    use futures_lite::prelude::*;
    use futures_lite::stream;

    #[test]
    fn merge_tuple_4() {
        block_on(async {
            let a = stream::once(1);
            let b = stream::once(2);
            let c = stream::once(3);
            let d = stream::once(4);
            let mut s = vec![a, b, c, d].merge();

            let mut counter = 0;
            while let Some(n) = s.next().await {
                counter += n;
            }
            assert_eq!(counter, 10);
        })
    }
}
