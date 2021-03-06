use super::Join as JoinTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

#[async_trait::async_trait(?Send)]
impl<T, const N: usize> JoinTrait for [T; N]
where
    T: Future,
{
    type Output = [T::Output; N];

    async fn join(self) -> Self::Output {
        Join {
            elems: self.map(MaybeDone::new),
        }
        .await
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
            let mut out: [MaybeUninit<F::Output>; N] = {
                // inlined version of unstable `MaybeUninit::uninit_array()`
                // TODO: replace with `MaybeUninit::uninit_array()` when it becomes stable
                unsafe { MaybeUninit::<[MaybeUninit<_>; N]>::uninit().assume_init() }
            };

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
