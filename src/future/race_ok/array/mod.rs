use super::RaceOk as RaceOkTrait;
use crate::utils::array_assume_init;
use crate::utils::iter_pin_mut;

use core::array;
use core::fmt;
use core::future::{Future, IntoFuture};
use core::mem::{self, MaybeUninit};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

mod error;

pub use error::AggregateError;

/// A future which waits for the first successful future to complete.
///
/// This `struct` is created by the [`race_ok`] method on the [`RaceOk`] trait. See
/// its documentation for more.
///
/// [`race_ok`]: crate::future::RaceOk::race_ok
/// [`RaceOk`]: crate::future::RaceOk
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
pub struct RaceOk<Fut, T, E, const N: usize>
where
    Fut: Future<Output = Result<T, E>>,
{
    #[pin]
    futures: [Fut; N],
    errors: [MaybeUninit<E>; N],
    completed: usize,
}

impl<Fut, T, E, const N: usize> fmt::Debug for RaceOk<Fut, T, E, N>
where
    Fut: Future<Output = Result<T, E>> + fmt::Debug,
    Fut::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.futures.iter()).finish()
    }
}

impl<Fut, T, E, const N: usize> Future for RaceOk<Fut, T, E, N>
where
    Fut: Future<Output = Result<T, E>>,
{
    type Output = Result<T, AggregateError<E, N>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let futures = iter_pin_mut(this.futures);

        for (fut, out) in futures.zip(this.errors.iter_mut()) {
            if let Poll::Ready(output) = fut.poll(cx) {
                match output {
                    Ok(ok) => return Poll::Ready(Ok(ok)),
                    Err(err) => {
                        *out = MaybeUninit::new(err);
                        *this.completed += 1;
                    }
                }
            }
        }

        let all_completed = *this.completed == N;
        if all_completed {
            let mut errors = array::from_fn(|_| MaybeUninit::uninit());
            mem::swap(&mut errors, this.errors);

            // SAFETY: we know that all futures are properly initialized because they're all completed
            let result = unsafe { array_assume_init(errors) };

            Poll::Ready(Err(AggregateError::new(result)))
        } else {
            Poll::Pending
        }
    }
}

impl<Fut, T, E, const N: usize> RaceOkTrait for [Fut; N]
where
    Fut: IntoFuture<Output = Result<T, E>>,
{
    type Output = T;
    type Error = AggregateError<E, N>;
    type Future = RaceOk<Fut::IntoFuture, T, E, N>;

    fn race_ok(self) -> Self::Future {
        RaceOk {
            futures: self.map(|fut| fut.into_future()),
            errors: array::from_fn(|_| MaybeUninit::uninit()),
            completed: 0,
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
