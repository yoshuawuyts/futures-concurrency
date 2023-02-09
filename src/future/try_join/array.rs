use super::TryJoin as TryJoinTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

/// Wait for all futures to complete successfully, or abort early on error.
///
/// This `struct` is created by the [`try_join`] method on the [`TryJoin`] trait. See
/// its documentation for more.
///
/// [`try_join`]: crate::future::TryJoin::try_join
/// [`TryJoin`]: crate::future::TryJoin
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
pub struct TryJoin<Fut, T, E, const N: usize>
where
    Fut: Future<Output = Result<T, E>>,
{
    elems: [MaybeDone<Fut>; N],
}

impl<Fut, T, E, const N: usize> fmt::Debug for TryJoin<Fut, T, E, N>
where
    Fut: Future<Output = Result<T, E>> + fmt::Debug,
    Fut::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.elems.iter()).finish()
    }
}

impl<Fut, T, E, const N: usize> Future for TryJoin<Fut, T, E, N>
where
    Fut: Future<Output = Result<T, E>>,
{
    type Output = Result<[T; N], E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_done = true;

        let this = self.project();

        for elem in this.elems.iter_mut() {
            // SAFETY: we don't ever move the pinned container here; we only pin project
            let mut elem = unsafe { Pin::new_unchecked(elem) };
            if elem.as_mut().poll(cx).is_pending() {
                all_done = false
            } else if let Some(err) = elem.take_err() {
                return Poll::Ready(Err(err));
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
                // SAFETY: we don't ever move the pinned container here; we only pin project
                let pin = unsafe { Pin::new_unchecked(el) };
                match pin.take_ok() {
                    Some(el) => out[i] = MaybeUninit::new(el),
                    // All futures are done and we iterate only once to take them so this is not
                    // reachable
                    None => unreachable!(),
                }
            }
            let result = unsafe { out.as_ptr().cast::<[T; N]>().read() };
            Poll::Ready(Ok(result))
        } else {
            Poll::Pending
        }
    }
}

impl<Fut, T, E, const N: usize> TryJoinTrait for [Fut; N]
where
    Fut: IntoFuture<Output = Result<T, E>>,
{
    type Output = [T; N];
    type Error = E;
    type Future = TryJoin<Fut::IntoFuture, T, E, N>;

    fn try_join(self) -> Self::Future {
        TryJoin {
            elems: self.map(|fut| MaybeDone::new(fut.into_future())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::future;
    use std::io::{self, Error, ErrorKind};

    #[test]
    fn all_ok() {
        futures_lite::future::block_on(async {
            let res: io::Result<_> = [future::ready(Ok("hello")), future::ready(Ok("world"))]
                .try_join()
                .await;
            assert_eq!(res.unwrap(), ["hello", "world"]);
        })
    }

    #[test]
    fn one_err() {
        futures_lite::future::block_on(async {
            let err = Error::new(ErrorKind::Other, "oh no");
            let res: io::Result<_> = [future::ready(Ok("hello")), future::ready(Err(err))]
                .try_join()
                .await;
            assert_eq!(res.unwrap_err().to_string(), String::from("oh no"));
        });
    }
}
