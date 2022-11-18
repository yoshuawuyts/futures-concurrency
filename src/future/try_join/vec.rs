use super::TryJoin as TryJoinTrait;
use crate::utils::iter_pin_mut;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::{Future, IntoFuture};
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::boxed::Box;
use std::vec::Vec;

/// Wait for all futures to complete successfully, or abort early on error.
///
/// This `struct` is created by the [`try_join`] method on the [`TryJoin`] trait. See
/// its documentation for more.
///
/// [`try_join`]: crate::future::TryJoin::try_join
/// [`TryJoin`]: crate::future::TryJoin
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct TryJoin<Fut, T, E>
where
    Fut: Future<Output = Result<T, E>>,
{
    elems: Pin<Box<[MaybeDone<Fut>]>>,
}

impl<Fut, T, E> fmt::Debug for TryJoin<Fut, T, E>
where
    Fut: Future<Output = Result<T, E>> + fmt::Debug,
    Fut::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.elems.iter()).finish()
    }
}

impl<Fut, T, E> Future for TryJoin<Fut, T, E>
where
    T: std::fmt::Debug,
    Fut: Future<Output = Result<T, E>>,
{
    type Output = Result<Vec<T>, E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_done = true;

        for mut elem in iter_pin_mut(self.elems.as_mut()) {
            if elem.as_mut().poll(cx).is_pending() {
                all_done = false
            } else if let Some(Err(_)) = elem.as_ref().output() {
                return Poll::Ready(Err(elem.take().unwrap().unwrap_err()));
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

impl<Fut, T, E> TryJoinTrait for Vec<Fut>
where
    T: std::fmt::Debug,
    Fut: IntoFuture<Output = Result<T, E>>,
{
    type Output = Vec<T>;
    type Error = E;
    type Future = TryJoin<Fut::IntoFuture, T, E>;

    fn try_join(self) -> Self::Future {
        let elems: Box<[_]> = self
            .into_iter()
            .map(|fut| MaybeDone::new(fut.into_future()))
            .collect();
        TryJoin {
            elems: elems.into(),
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
            let res: io::Result<_> = vec![future::ready(Ok("hello")), future::ready(Ok("world"))]
                .try_join()
                .await;
            assert_eq!(res.unwrap(), vec!["hello", "world"]);
        })
    }

    #[test]
    fn one_err() {
        futures_lite::future::block_on(async {
            let err = Error::new(ErrorKind::Other, "oh no");
            let res: io::Result<_> = vec![future::ready(Ok("hello")), future::ready(Err(err))]
                .try_join()
                .await;
            assert_eq!(res.unwrap_err().to_string(), String::from("oh no"));
        });
    }
}
