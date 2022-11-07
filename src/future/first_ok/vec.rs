use super::FirstOk as FirstOkTrait;
use crate::utils::iter_pin_mut;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::{Future, IntoFuture};
use core::mem;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::boxed::Box;
use std::ops::Deref;
use std::ops::DerefMut;
use std::vec::Vec;

/// A collection of errors.
#[repr(transparent)]
pub struct AggregateError<E> {
    inner: Vec<E>,
}

impl<E> AggregateError<E> {
    fn new(inner: Vec<E>) -> Self {
        Self { inner }
    }
}

impl<E: fmt::Debug> fmt::Debug for AggregateError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for err in &self.inner {
            list.entry(err);
        }
        list.finish()
    }
}

impl<E: fmt::Debug> fmt::Display for AggregateError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<E> Deref for AggregateError<E> {
    type Target = Vec<E>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<E> DerefMut for AggregateError<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<E: fmt::Debug> std::error::Error for AggregateError<E> {}

#[async_trait::async_trait(?Send)]
impl<Fut, T, E> FirstOkTrait for Vec<Fut>
where
    T: fmt::Debug,
    E: fmt::Debug,
    Fut: IntoFuture<Output = Result<T, E>>,
{
    type Output = T;
    type Error = AggregateError<E>;

    async fn first_ok(self) -> Result<Self::Output, Self::Error> {
        let elems: Box<[_]> = self
            .into_iter()
            .map(|fut| MaybeDone::new(fut.into_future()))
            .collect();
        FirstOk {
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
pub(super) struct FirstOk<Fut, T, E>
where
    Fut: Future<Output = Result<T, E>>,
{
    elems: Pin<Box<[MaybeDone<Fut>]>>,
}

impl<Fut, T, E> fmt::Debug for FirstOk<Fut, T, E>
where
    Fut: Future<Output = Result<T, E>> + fmt::Debug,
    Fut::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FirstOk")
            .field("elems", &self.elems)
            .finish()
    }
}

impl<Fut, T, E> Future for FirstOk<Fut, T, E>
where
    T: std::fmt::Debug,
    E: fmt::Debug,
    Fut: Future<Output = Result<T, E>>,
{
    type Output = Result<T, AggregateError<E>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_done = true;

        for mut elem in iter_pin_mut(self.elems.as_mut()) {
            if let Poll::Pending = elem.as_mut().poll(cx) {
                all_done = false
            } else if let Some(Ok(_)) = elem.as_ref().output() {
                return Poll::Ready(Ok(elem.take().unwrap().unwrap()));
            }
        }

        if all_done {
            let mut elems = mem::replace(&mut self.elems, Box::pin([]));
            let result: Vec<E> = iter_pin_mut(elems.as_mut())
                .map(|e| e.take().unwrap().unwrap_err())
                .collect();
            Poll::Ready(Err(AggregateError::new(result)))
        } else {
            Poll::Pending
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
            let res: Result<&str, AggregateError<Error>> =
                vec![future::ready(Ok("hello")), future::ready(Ok("world"))]
                    .first_ok()
                    .await;
            assert!(res.is_ok());
        })
    }

    #[test]
    fn one_err() {
        futures_lite::future::block_on(async {
            let err = Error::new(ErrorKind::Other, "oh no");
            let res: Result<&str, AggregateError<Error>> =
                vec![future::ready(Ok("hello")), future::ready(Err(err))]
                    .first_ok()
                    .await;
            assert_eq!(res.unwrap(), "hello");
        });
    }

    #[test]
    fn all_err() {
        futures_lite::future::block_on(async {
            let err1 = Error::new(ErrorKind::Other, "oops");
            let err2 = Error::new(ErrorKind::Other, "oh no");
            let res: Result<&str, AggregateError<Error>> =
                vec![future::ready(Err(err1)), future::ready(Err(err2))]
                    .first_ok()
                    .await;
            let errs = res.unwrap_err();
            assert_eq!(errs[0].to_string(), "oops");
            assert_eq!(errs[1].to_string(), "oh no");
        });
    }
}
