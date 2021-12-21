use super::*;
use crate::utils::Fuse;

use futures_core::Stream;
use pin_project::pin_project;

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
