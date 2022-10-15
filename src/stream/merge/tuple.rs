use super::Merge as MergeTrait;
use crate::stream::IntoStream;
use crate::utils;

use crate::stream::Stream;
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

impl<T, A, B> MergeTrait for (A, B)
where
    A: IntoStream<Item = T>,
    B: IntoStream<Item = T>,
{
    type Item = T;
    type Stream = Merge2<T, A::IntoStream, B::IntoStream>;

    fn merge(self) -> Self::Stream {
        Merge2::new((self.0.into_stream(), self.1.into_stream()))
    }
}

#[derive(Debug)]
#[pin_project::pin_project]
pub struct Merge2<T, A, B>
where
    A: Stream<Item = T>,
    B: Stream<Item = T>,
{
    streams: (A, B),
}

impl<T, A, B> Merge2<T, A, B>
where
    A: Stream<Item = T>,
    B: Stream<Item = T>,
{
    pub(crate) fn new(streams: (A, B)) -> Self {
        Self { streams }
    }
}

impl<T, A, B> Stream for Merge2<T, A, B>
where
    A: Stream<Item = T>,
    B: Stream<Item = T>,
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

impl<T, A, B, C> MergeTrait for (A, B, C)
where
    A: IntoStream<Item = T>,
    B: IntoStream<Item = T>,
    C: IntoStream<Item = T>,
{
    type Item = T;
    type Stream = Merge3<T, A::IntoStream, B::IntoStream, C::IntoStream>;

    fn merge(self) -> Self::Stream {
        Merge3::new((
            self.0.into_stream(),
            self.1.into_stream(),
            self.2.into_stream(),
        ))
    }
}

#[derive(Debug)]
#[pin_project::pin_project]
pub struct Merge3<T, A, B, C>
where
    A: Stream<Item = T>,
    B: Stream<Item = T>,
    C: Stream<Item = T>,
{
    streams: (A, B, C),
}

impl<T, A, B, C> Merge3<T, A, B, C>
where
    A: Stream<Item = T>,
    B: Stream<Item = T>,
    C: Stream<Item = T>,
{
    pub(crate) fn new(streams: (A, B, C)) -> Self {
        Self { streams }
    }
}

impl<T, A, B, C> Stream for Merge3<T, A, B, C>
where
    A: Stream<Item = T>,
    B: Stream<Item = T>,
    C: Stream<Item = T>,
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
impl<T, A, B, C, D> MergeTrait for (A, B, C, D)
where
    A: IntoStream<Item = T>,
    B: IntoStream<Item = T>,
    C: IntoStream<Item = T>,
    D: IntoStream<Item = T>,
{
    type Item = T;
    type Stream = Merge4<T, A::IntoStream, B::IntoStream, C::IntoStream, D::IntoStream>;

    fn merge(self) -> Self::Stream {
        Merge4::new((
            self.0.into_stream(),
            self.1.into_stream(),
            self.2.into_stream(),
            self.3.into_stream(),
        ))
    }
}

#[derive(Debug)]
#[pin_project::pin_project]
pub struct Merge4<T, A, B, C, D>
where
    A: Stream<Item = T>,
    B: Stream<Item = T>,
    C: Stream<Item = T>,
    D: Stream<Item = T>,
{
    streams: (A, B, C, D),
}

impl<T, A, B, C, D> Merge4<T, A, B, C, D>
where
    A: Stream<Item = T>,
    B: Stream<Item = T>,
    C: Stream<Item = T>,
    D: Stream<Item = T>,
{
    pub(crate) fn new(streams: (A, B, C, D)) -> Self {
        Self { streams }
    }
}

impl<T, A, B, C, D> Stream for Merge4<T, A, B, C, D>
where
    A: Stream<Item = T>,
    B: Stream<Item = T>,
    C: Stream<Item = T>,
    D: Stream<Item = T>,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        // SAFETY: we're manually projecting the tuple fields here.
        let s0 = unsafe { Pin::new_unchecked(&mut this.streams.0) };
        let s1 = unsafe { Pin::new_unchecked(&mut this.streams.1) };
        let s2 = unsafe { Pin::new_unchecked(&mut this.streams.2) };
        let s3 = unsafe { Pin::new_unchecked(&mut this.streams.3) };
        match utils::random(24) {
            // s0 first
            0 => poll_in_order!(cx, s0, s1, s2, s3),
            1 => poll_in_order!(cx, s0, s1, s3, s2),
            2 => poll_in_order!(cx, s0, s2, s1, s3),
            3 => poll_in_order!(cx, s0, s2, s3, s1),
            4 => poll_in_order!(cx, s0, s3, s1, s2),
            5 => poll_in_order!(cx, s0, s3, s2, s1),
            // s1 first
            6 => poll_in_order!(cx, s1, s0, s2, s3),
            7 => poll_in_order!(cx, s1, s0, s3, s2),
            8 => poll_in_order!(cx, s1, s2, s0, s3),
            9 => poll_in_order!(cx, s1, s2, s3, s0),
            10 => poll_in_order!(cx, s1, s3, s0, s2),
            11 => poll_in_order!(cx, s1, s3, s2, s0),
            // s2 first
            12 => poll_in_order!(cx, s2, s0, s1, s3),
            13 => poll_in_order!(cx, s2, s0, s3, s1),
            14 => poll_in_order!(cx, s2, s1, s0, s3),
            15 => poll_in_order!(cx, s2, s1, s3, s0),
            16 => poll_in_order!(cx, s2, s3, s0, s1),
            17 => poll_in_order!(cx, s2, s3, s1, s0),
            // s3 first
            18 => poll_in_order!(cx, s3, s0, s1, s2),
            19 => poll_in_order!(cx, s3, s0, s2, s1),
            20 => poll_in_order!(cx, s3, s1, s0, s2),
            21 => poll_in_order!(cx, s3, s1, s2, s0),
            22 => poll_in_order!(cx, s3, s2, s0, s1),
            23 => poll_in_order!(cx, s3, s2, s1, s0),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_tuple_4() {
        use crate::stream;
        use futures_lite::future::block_on;

        block_on(async {
            let a = stream::once(1);
            let b = stream::once(2);
            let c = stream::once(3);
            let d = stream::once(4);
            let mut s = (a, b, c, d).merge();

            let mut counter = 0;
            while let Some(n) = s.next().await {
                counter += n;
            }
            assert_eq!(counter, 10);
        })
    }
}
