use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

/// First attempt to poll the first future, then poll the second future.
pub(crate) fn poll_next_in_order<F, S, I>(
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
