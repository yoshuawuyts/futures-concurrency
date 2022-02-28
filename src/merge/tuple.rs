use crate::stream::IntoStream;
use crate::utils;
use crate::Merge as MergeTrait;

use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

macro_rules! poll_in_order {
    ($cx:expr, $stream:expr) => { $stream.poll_next($cx) };
    ($cx:expr, $stream:expr, $($next:tt),*) => {{
        let mut pending = false;
        match $stream.poll_next($cx) {
            Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
            Poll::Pending => { pending = true; }
            Poll::Ready(None) => {},
        }
        match poll_in_order!($cx, $($next),*) {
            Poll::Ready(None) if pending => Poll::Pending,
            other => other,
        }
    }};
}

impl<T, S0, S1> MergeTrait for (S0, S1)
where
    S0: IntoStream<Item = T>,
    S1: IntoStream<Item = T>,
{
    type Item = T;
    type Stream = Merge2<T, S0::IntoStream, S1::IntoStream>;

    fn merge(self) -> Self::Stream {
        Merge2::new((self.0.into_stream(), self.1.into_stream()))
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
pub struct Merge2<T, S0, S1>
where
    S0: Stream<Item = T>,
    S1: Stream<Item = T>,
{
    streams: (S0, S1),
}

impl<T, S0, S1> Merge2<T, S0, S1>
where
    S0: Stream<Item = T>,
    S1: Stream<Item = T>,
{
    pub(crate) fn new(streams: (S0, S1)) -> Self {
        Self { streams }
    }
}

impl<T, S0, S1> Stream for Merge2<T, S0, S1>
where
    S0: Stream<Item = T>,
    S1: Stream<Item = T>,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        // SAFETY: we're manually projecting the tuple fields here.
        let s0 = unsafe { Pin::new_unchecked(&mut this.streams.0) };
        let s1 = unsafe { Pin::new_unchecked(&mut this.streams.1) };
        match utils::random(2) {
            0 => poll_in_order!(cx, s0, s1),
            1 => poll_in_order!(cx, s1, s0),
            _ => unreachable!(),
        }
    }
}

// TODO: automate this!

impl<T, S0, S1, S2> MergeTrait for (S0, S1, S2)
where
    S0: IntoStream<Item = T>,
    S1: IntoStream<Item = T>,
    S2: IntoStream<Item = T>,
{
    type Item = T;
    type Stream = Merge3<T, S0::IntoStream, S1::IntoStream, S2::IntoStream>;

    fn merge(self) -> Self::Stream {
        Merge3::new((
            self.0.into_stream(),
            self.1.into_stream(),
            self.2.into_stream(),
        ))
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
pub struct Merge3<T, S0, S1, S2>
where
    S0: Stream<Item = T>,
    S1: Stream<Item = T>,
    S2: Stream<Item = T>,
{
    streams: (S0, S1, S2),
}

impl<T, S0, S1, S2> Merge3<T, S0, S1, S2>
where
    S0: Stream<Item = T>,
    S1: Stream<Item = T>,
    S2: Stream<Item = T>,
{
    pub(crate) fn new(streams: (S0, S1, S2)) -> Self {
        Self { streams }
    }
}

impl<T, S0, S1, S2> Stream for Merge3<T, S0, S1, S2>
where
    S0: Stream<Item = T>,
    S1: Stream<Item = T>,
    S2: Stream<Item = T>,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        // SAFETY: we're manually projecting the tuple fields here.
        let s0 = unsafe { Pin::new_unchecked(&mut this.streams.0) };
        let s1 = unsafe { Pin::new_unchecked(&mut this.streams.1) };
        let s2 = unsafe { Pin::new_unchecked(&mut this.streams.2) };
        match utils::random(6) {
            0 => poll_in_order!(cx, s0, s1, s2),
            1 => poll_in_order!(cx, s0, s2, s1),
            2 => poll_in_order!(cx, s1, s0, s2),
            3 => poll_in_order!(cx, s1, s2, s0),
            4 => poll_in_order!(cx, s2, s0, s1),
            5 => poll_in_order!(cx, s2, s1, s0),
            _ => unreachable!(),
        }
    }
}
