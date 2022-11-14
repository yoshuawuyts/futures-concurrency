use super::Merge as MergeTrait;
use crate::stream::IntoStream;
use crate::utils::{self, PollState};

use core::array;
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
    streams: [S; N],
    rng: utils::RandomGenerator,
    poll_state: [PollState; N],
    done: bool,
}

impl<S, const N: usize> Merge<S, N>
where
    S: Stream,
{
    pub(crate) fn new(streams: [S; N]) -> Self {
        Self {
            streams,
            rng: utils::RandomGenerator::new(),
            poll_state: array::from_fn(|_| PollState::default()),
            done: false,
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

        // Iterate over our streams one-by-one. If a stream yields a value,
        // we exit early. By default we'll return `Poll::Ready(None)`, but
        // this changes if we encounter a `Poll::Pending`.
        let mut res = Poll::Ready(None);
        let index = this.rng.generate(N as u32) as usize;

        for index in (0..N).map(|pos| (index + pos).wrapping_rem(N)) {
            if this.poll_state[index].is_consumed() {
                continue;
            }

            let stream = utils::get_pin_mut(this.streams.as_mut(), index).unwrap();
            match stream.poll_next(cx) {
                Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
                Poll::Ready(None) => {
                    this.poll_state[index] = PollState::Consumed;
                    continue;
                }
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
            let mut s = [a, b, c, d].merge();

            let mut counter = 0;
            while let Some(n) = s.next().await {
                counter += n;
            }
            assert_eq!(counter, 10);
        })
    }
}
