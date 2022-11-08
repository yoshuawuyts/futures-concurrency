use super::RaceOk as RaceOkTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

mod error;

pub use error::AggregateError;

/// Waits for two similarly-typed futures to complete.
///
/// Wait for the first future to complete.
///
/// This `struct` is created by the [`try_race`] method on the [`TryRace`] trait. See
/// its documentation for more.
///
/// [`try_race`]: crate::future::TryRace::try_race
/// [`TryRace`]: crate::future::TryRace
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
pub struct RaceOk<Fut, T, E, const N: usize>
where
    T: fmt::Debug,
    Fut: Future<Output = Result<T, E>>,
{
    elems: [MaybeDone<Fut>; N],
}

impl<Fut, T, E, const N: usize> fmt::Debug for RaceOk<Fut, T, E, N>
where
    Fut: Future<Output = Result<T, E>> + fmt::Debug,
    Fut::Output: fmt::Debug,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.elems.iter()).finish()
    }
}

impl<Fut, T, E, const N: usize> Future for RaceOk<Fut, T, E, N>
where
    T: fmt::Debug,
    Fut: Future<Output = Result<T, E>>,
    E: fmt::Debug,
{
    type Output = Result<T, AggregateError<E, N>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_done = true;

        let this = self.project();

        for elem in this.elems.iter_mut() {
            // SAFETY: we don't ever move the pinned container here; we only pin project
            let mut elem = unsafe { Pin::new_unchecked(elem) };
            if let Poll::Pending = elem.as_mut().poll(cx) {
                all_done = false
            } else if let Some(Ok(_)) = elem.as_ref().output() {
                return Poll::Ready(Ok(elem.take().unwrap().unwrap()));
            }
        }

        if all_done {
            use core::mem::MaybeUninit;

            // Create the result array based on the indices
            let mut out: [MaybeUninit<E>; N] = {
                // inlined version of unstable `MaybeUninit::uninit_array()`
                // TODO: replace with `MaybeUninit::uninit_array()` when it becomes stable
                unsafe { MaybeUninit::<[MaybeUninit<_>; N]>::uninit().assume_init() }
            };

            // NOTE: this clippy attribute can be removed once we can `collect` into `[usize; K]`.
            #[allow(clippy::clippy::needless_range_loop)]
            for (i, el) in this.elems.iter_mut().enumerate() {
                // SAFETY: we don't ever move the pinned container here; we only pin project
                let el = unsafe { Pin::new_unchecked(el) }
                    .take()
                    .unwrap()
                    .unwrap_err();
                out[i] = MaybeUninit::new(el);
            }
            let result = unsafe { out.as_ptr().cast::<[E; N]>().read() };
            Poll::Ready(Err(AggregateError::new(result)))
        } else {
            Poll::Pending
        }
    }
}

impl<Fut, T, E, const N: usize> RaceOkTrait for [Fut; N]
where
    T: fmt::Debug,
    Fut: IntoFuture<Output = Result<T, E>>,
    E: fmt::Debug,
{
    type Output = T;
    type Error = AggregateError<E, N>;
    type Future = RaceOk<Fut::IntoFuture, T, E, N>;

    fn race_ok(self) -> Self::Future {
        RaceOk {
            elems: self.map(|fut| MaybeDone::new(fut.into_future())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::future;
    use std::io::{Error, ErrorKind};

    #[test]
    fn all_ok() {
        futures_lite::future::block_on(async {
            let res: Result<&str, AggregateError<Error, 2>> =
                [future::ready(Ok("hello")), future::ready(Ok("world"))]
                    .race_ok()
                    .await;
            assert!(res.is_ok());
        })
    }

    #[test]
    fn one_err() {
        futures_lite::future::block_on(async {
            let err = Error::new(ErrorKind::Other, "oh no");
            let res: Result<&str, AggregateError<Error, 2>> =
                [future::ready(Ok("hello")), future::ready(Err(err))]
                    .race_ok()
                    .await;
            assert_eq!(res.unwrap(), "hello");
        });
    }

    #[test]
    fn all_err() {
        futures_lite::future::block_on(async {
            let err1 = Error::new(ErrorKind::Other, "oops");
            let err2 = Error::new(ErrorKind::Other, "oh no");
            let res: Result<&str, AggregateError<Error, 2>> =
                [future::ready(Err(err1)), future::ready(Err(err2))]
                    .race_ok()
                    .await;
            let errs = res.unwrap_err();
            assert_eq!(errs[0].to_string(), "oops");
            assert_eq!(errs[1].to_string(), "oh no");
        });
    }
}
