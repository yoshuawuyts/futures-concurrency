use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;
use pin_project::pin_project;

/// Extend `Stream` with concurrency methods.
pub trait StreamExt: Stream {
    /// Combines multiple streams into a single stream of all their outputs.
    ///
    /// Items are yielded as soon as they're received, and the stream continues
    /// yield until both streams have been exhausted. The output ordering
    /// between streams is not guaranteed.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_lite::prelude::*;
    /// use futures_lite::future::block_on;
    /// use futures_lite::stream;
    /// use futures_concurrency::prelude::*;
    ///
    /// fn main() {
    ///     block_on(async {
    ///         let a = stream::once(1u8);
    ///         let b = stream::once(2u8);
    ///         let c = stream::once(3u8);
    ///         let s = a.merge(b).merge(c);
    ///
    ///         let mut buf = vec![];
    ///         s.for_each(|n| buf.push(n)).await;
    ///         buf.sort_unstable();
    ///         assert_eq!(&buf, &[1u8, 2u8, 3u8]);
    ///     })
    /// }
    /// ```
    fn merge<U>(self, other: U) -> Merge<Self, U>
    where
        Self: Sized,
        U: Stream<Item = Self::Item> + Sized,
    {
        Merge::new(self, other)
    }
}

impl<S> StreamExt for S where S: Stream {}

/// A stream that merges two other streams into a single stream.
///
/// This `struct` is created by the [`merge`] method on [`Stream`]. See its
/// documentation for more.
///
/// [`merge`]: trait.Stream.html#method.merge
/// [`Stream`]: trait.Stream.html
#[pin_project]
#[derive(Debug)]
pub struct Merge<L, R> {
    #[pin]
    left: Fuse<L>,
    #[pin]
    right: Fuse<R>,
}

impl<L: Stream, R: Stream> Merge<L, R> {
    pub(crate) fn new(left: L, right: R) -> Self {
        Self {
            left: Fuse::new(left),
            right: Fuse::new(right),
        }
    }
}

impl<L, R, T> Stream for Merge<L, R>
where
    L: Stream<Item = T>,
    R: Stream<Item = T>,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        if random(2) == 0 {
            poll_next_in_order(this.left, this.right, cx)
        } else {
            poll_next_in_order(this.right, this.left, cx)
        }
    }
}

fn poll_next_in_order<F, S, I>(
    first: Pin<&mut F>,
    second: Pin<&mut S>,
    cx: &mut Context<'_>,
) -> Poll<Option<I>>
where
    F: Stream<Item = I>,
    S: Stream<Item = I>,
{
    match first.poll_next(cx) {
        Poll::Ready(None) => second.poll_next(cx),
        Poll::Ready(item) => Poll::Ready(item),
        Poll::Pending => match second.poll_next(cx) {
            Poll::Ready(None) | Poll::Pending => Poll::Pending,
            Poll::Ready(item) => Poll::Ready(item),
        },
    }
}

/// Generates a random number in `0..n`.
pub(crate) fn random(n: u32) -> u32 {
    use std::cell::Cell;
    use std::num::Wrapping;

    thread_local! {
        static RNG: Cell<Wrapping<u32>> = {
            // Take the address of a local value as seed.
            let mut x = 0i32;
            let r = &mut x;
            let addr = r as *mut i32 as usize;
            Cell::new(Wrapping(addr as u32))
        }
    }

    RNG.with(|rng| {
        // This is the 32-bit variant of Xorshift.
        //
        // Source: https://en.wikipedia.org/wiki/Xorshift
        let mut x = rng.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        rng.set(x);

        // This is a fast alternative to `x % n`.
        //
        // Author: Daniel Lemire
        // Source: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/
        ((u64::from(x.0)).wrapping_mul(u64::from(n)) >> 32) as u32
    })
}

/// A stream that yields `None` forever after the underlying stream yields `None` once.
///
/// This `struct` is created by the [`fuse`] method on [`Stream`]. See its
/// documentation for more.
///
/// [`fuse`]: trait.Stream.html#method.fuse
/// [`Stream`]: trait.Stream.html
#[pin_project]
#[derive(Clone, Debug)]
pub struct Fuse<S> {
    #[pin]
    pub(crate) stream: S,
    pub(crate) done: bool,
}

impl<S> Fuse<S> {
    pub(super) fn new(stream: S) -> Self {
        Self {
            stream,
            done: false,
        }
    }
}

impl<S: Stream> Stream for Fuse<S> {
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        let this = self.project();
        if *this.done {
            Poll::Ready(None)
        } else {
            let next = futures_core::ready!(this.stream.poll_next(cx));
            if next.is_none() {
                *this.done = true;
            }
            Poll::Ready(next)
        }
    }
}
