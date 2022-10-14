use super::TryMerge as TryMergeTrait;
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
impl<Fut, T, E> TryMergeTrait for Vec<Fut>
where
    T: std::fmt::Debug,
    Fut: IntoFuture<Output = Result<T, E>>,
{
    type Output = Vec<T>;
    type Error = E;
    async fn try_merge(self) -> Result<Self::Output, Self::Error> {
        let elems: Box<[_]> = self
            .into_iter()
            .map(|fut| MaybeDone::new(fut.into_future()))
            .collect();
        TryJoin {
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
pub(super) struct TryJoin<Fut, T, E>
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
        f.debug_struct("TryJoin")
            .field("elems", &self.elems)
            .finish()
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
            if let Poll::Pending = elem.as_mut().poll(cx) {
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

#[cfg(test)]
mod test {
    use super::*;
    use std::future;
    use std::io::{self, Error, ErrorKind};

    #[test]
    fn all_ok() {
        futures_lite::future::block_on(async {
            let res: io::Result<_> = vec![future::ready(Ok("hello")), future::ready(Ok("world"))]
                .try_merge()
                .await;
            assert_eq!(res.unwrap(), vec!["hello", "world"]);
        })
    }

    #[test]
    fn one_err() {
        futures_lite::future::block_on(async {
            let err = Error::new(ErrorKind::Other, "oh no");
            let res: io::Result<_> = vec![future::ready(Ok("hello")), future::ready(Err(err))]
                .try_merge()
                .await;
            assert_eq!(res.unwrap_err().to_string(), String::from("oh no"));
        });
    }
}
