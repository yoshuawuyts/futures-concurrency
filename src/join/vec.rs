use crate::iter_pin_mut;

use super::Join as JoinTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::Future;
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::boxed::Box;
use std::vec::Vec;

impl<T> JoinTrait for Vec<T>
where
    T: Future,
{
    type Output = Vec<T::Output>;
    type Future = Join<T>;

    fn join(self) -> Self::Future {
        let elems: Box<[_]> = self.into_iter().map(MaybeDone::new).collect();
        Join {
            elems: elems.into(),
        }
    }
}

/// Waits for two similarly-typed futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the
/// futures once both complete.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Join<F>
where
    F: Future,
{
    elems: Pin<Box<[MaybeDone<F>]>>,
}

impl<F> fmt::Debug for Join<F>
where
    F: Future + fmt::Debug,
    F::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Join").field("elems", &self.elems).finish()
    }
}

impl<F> Future for Join<F>
where
    F: Future,
{
    type Output = Vec<F::Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_done = true;

        for elem in iter_pin_mut(self.elems.as_mut()) {
            if elem.poll(cx).is_pending() {
                all_done = false;
            }
        }

        if all_done {
            let mut elems = mem::replace(&mut self.elems, Box::pin([]));
            let result = iter_pin_mut(elems.as_mut())
                .map(|e| e.take().unwrap())
                .collect();
            Poll::Ready(result)
        } else {
            Poll::Pending
        }
    }
}
