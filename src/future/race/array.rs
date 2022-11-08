use super::Race as RaceTrait;

use core::fmt;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

/// Wait for the first future to complete.
///
/// This `struct` is created by the [`race`] method on the [`Race`] trait. See
/// its documentation for more.
///
/// [`race`]: crate::future::Race::race
/// [`Race`]: crate::future::Race
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
pub struct Race<Fut, const N: usize>
where
    Fut: Future,
{
    futs: [Fut; N],
    done: bool,
}

impl<Fut, const N: usize> fmt::Debug for Race<Fut, N>
where
    Fut: Future + fmt::Debug,
    Fut::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.futs.iter()).finish()
    }
}

impl<Fut, const N: usize> Future for Race<Fut, N>
where
    Fut: Future,
{
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        assert!(
            !*this.done,
            "Futures must not be polled after being completed"
        );
        for fut in this.futs {
            let fut = unsafe { Pin::new_unchecked(fut) };
            if let Poll::Ready(output) = Future::poll(fut, cx) {
                *this.done = true;
                return Poll::Ready(output);
            }
        }
        Poll::Pending
    }
}

impl<Fut, const N: usize> RaceTrait for [Fut; N]
where
    Fut: IntoFuture,
{
    type Output = Fut::Output;
    type Future = Race<Fut::IntoFuture, N>;

    fn race(self) -> Self::Future {
        Race {
            futs: self.map(|fut| fut.into_future()),
            done: false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::future;

    // NOTE: we should probably poll in random order.
    #[test]
    fn no_fairness() {
        futures_lite::future::block_on(async {
            let res = [future::ready("hello"), future::ready("world")]
                .race()
                .await;
            assert_eq!(res, "hello");
        });
    }
}
