use super::Join as JoinTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

impl<T, const N: usize> JoinTrait for [T; N]
where
    T: Future,
{
    type Output = [T::Output; N];
    type Future = Join<T, N>;

    fn join(self) -> Self::Future {
        Join {
            elems: self.map(MaybeDone::new),
        }
    }
}

/// Waits for two similarly-typed futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the
/// futures once both complete.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
pub struct Join<F, const N: usize>
where
    F: Future,
{
    elems: [MaybeDone<F>; N],
}

impl<F, const N: usize> fmt::Debug for Join<F, N>
where
    F: Future + fmt::Debug,
    F::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Join").field("elems", &self.elems).finish()
    }
}

impl<F, const N: usize> Future for Join<F, N>
where
    F: Future,
{
    type Output = [F::Output; N];

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
            use core::mem::MaybeUninit;

            // Create the result array based on the indices
            let mut out: [MaybeUninit<F::Output>; N] = MaybeUninit::uninit_array();

            // NOTE: this clippy attribute can be removed once we can `collect` into `[usize; K]`.
            #[allow(clippy::clippy::needless_range_loop)]
            for (i, el) in this.elems.iter_mut().enumerate() {
                let el = unsafe { Pin::new_unchecked(el) }.take().unwrap();
                out[i] = MaybeUninit::new(el);
            }
            let result = unsafe { out.as_ptr().cast::<[F::Output; N]>().read() };
            Poll::Ready(result)
        } else {
            Poll::Pending
        }
    }
}
