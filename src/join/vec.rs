use super::Join as JoinTrait;
use crate::utils::iter_pin_mut;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::{Future, IntoFuture};
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::boxed::Box;
use std::vec::Vec;

#[async_trait::async_trait(?Send)]
impl<Fut> JoinTrait for Vec<Fut>
where
    Fut: IntoFuture,
{
    type Output = Vec<Fut::Output>;

    async fn join(self) -> Self::Output {
        let elems: Box<[_]> = self
            .into_iter()
            .map(|fut| MaybeDone::new(fut.into_future()))
            .collect();
        Join {
            elems: elems.into(),
        }
        .await
    }
}

/// Waits for two similarly-typed futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the
/// futures once both complete.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Join<Fut>
where
    Fut: Future,
{
    elems: Pin<Box<[MaybeDone<Fut>]>>,
}

impl<Fut> fmt::Debug for Join<Fut>
where
    Fut: Future + fmt::Debug,
    Fut::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Join").field("elems", &self.elems).finish()
    }
}

impl<Fut> Future for Join<Fut>
where
    Fut: Future,
{
    type Output = Vec<Fut::Output>;

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
