use super::Join as JoinTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

/// Waits for two similarly-typed futures to complete.
///
/// This `struct` is created by the [`join`] method on the [`Join`] trait. See
/// its documentation for more.
///
/// [`join`]: crate::future::Join::join
/// [`Join`]: crate::future::Join
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
pub struct Join<Fut, const N: usize>
where
    Fut: Future,
{
    pub(crate) elems: [MaybeDone<Fut>; N],
}

impl<Fut, const N: usize> fmt::Debug for Join<Fut, N>
where
    Fut: Future + fmt::Debug,
    Fut::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.elems.iter()).finish()
    }
}

impl<Fut, const N: usize> Future for Join<Fut, N>
where
    Fut: Future,
{
    type Output = [Fut::Output; N];

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_done = true;

        let this = self.project();

        for elem in this.elems.iter_mut() {
            let elem = unsafe { Pin::new_unchecked(elem) };
            if elem.poll(cx).is_pending() {
                all_done = false;
            }
        }

        if all_done {
            use core::array;
            use core::mem::MaybeUninit;

            // Create the result array based on the indices
            // TODO: replace with `MaybeUninit::uninit_array()` when it becomes stable
            let mut out: [_; N] = array::from_fn(|_| MaybeUninit::uninit());

            // NOTE: this clippy attribute can be removed once we can `collect` into `[usize; K]`.
            #[allow(clippy::needless_range_loop)]
            for (i, el) in this.elems.iter_mut().enumerate() {
                let el = unsafe { Pin::new_unchecked(el) }.take().unwrap();
                out[i] = MaybeUninit::new(el);
            }
            let result = unsafe { out.as_ptr().cast::<[Fut::Output; N]>().read() };
            Poll::Ready(result)
        } else {
            Poll::Pending
        }
    }
}

impl<Fut, const N: usize> JoinTrait for [Fut; N]
where
    Fut: IntoFuture,
{
    type Output = [Fut::Output; N];
    type Future = Join<Fut::IntoFuture, N>;
    fn join(self) -> Self::Future {
        Join {
            elems: self.map(|fut| MaybeDone::new(fut.into_future())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::future;

    #[test]
    fn smoke() {
        futures_lite::future::block_on(async {
            let res = [future::ready("hello"), future::ready("world")]
                .join()
                .await;
            assert_eq!(res, ["hello", "world"]);
        });
    }
}
