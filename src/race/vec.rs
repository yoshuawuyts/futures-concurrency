use super::Race as RaceTrait;

use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

#[async_trait::async_trait(?Send)]
impl<T> RaceTrait for Vec<T>
where
    T: Future,
{
    type Output = T::Output;

    async fn race(self) -> Self::Output {
        Race {
            futs: self,
            done: false,
        }
        .await
    }
}

/// Waits for multiple similarly-typed futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the
/// first future which completes.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
pub struct Race<F>
where
    F: Future,
{
    futs: Vec<F>,
    done: bool,
}

impl<F> fmt::Debug for Race<F>
where
    F: Future + fmt::Debug,
    F::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Race").field("futs", &self.futs).finish()
    }
}

impl<F> Future for Race<F>
where
    F: Future,
{
    type Output = F::Output;

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

#[cfg(test)]
mod test {
    use super::*;
    use std::future;

    // NOTE: we should probably poll in random order.
    #[test]
    fn no_fairness() {
        async_io::block_on(async {
            let res = vec![future::ready("hello"), future::ready("world")]
                .race()
                .await;
            assert_eq!(res, "hello");
        });
    }
}
