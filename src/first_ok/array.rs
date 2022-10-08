use super::FirstOk as FirstOkTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::ops::{Deref, DerefMut};

use pin_project::pin_project;

/// A collection of errors.
#[repr(transparent)]
pub struct AggregateError<T, const N: usize> {
    inner: [T; N],
}

impl<T, const N: usize> AggregateError<T, N> {
    fn new(inner: [T; N]) -> Self {
        Self { inner }
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for AggregateError<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for err in self.inner.as_ref() {
            list.entry(err);
        }
        list.finish()
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Display for AggregateError<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<T, const N: usize> Deref for AggregateError<T, N> {
    type Target = [T; N];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T, const N: usize> DerefMut for AggregateError<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: fmt::Debug, const N: usize> std::error::Error for AggregateError<T, N> {}

#[async_trait::async_trait(?Send)]
impl<F, T, E, const N: usize> FirstOkTrait for [F; N]
where
    T: fmt::Debug,
    F: Future<Output = Result<T, E>>,
    E: fmt::Debug,
{
    type Output = T;
    type Error = AggregateError<E, N>;

    async fn first_ok(self) -> Result<Self::Output, Self::Error> {
        FirstOk {
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
pub struct FirstOk<F, T, E, const N: usize>
where
    T: fmt::Debug,
    F: Future<Output = Result<T, E>>,
{
    elems: [MaybeDone<F>; N],
}

impl<F, T, E, const N: usize> fmt::Debug for FirstOk<F, T, E, N>
where
    F: Future<Output = Result<T, E>> + fmt::Debug,
    F::Output: fmt::Debug,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Join").field("elems", &self.elems).finish()
    }
}

impl<F, T, E, const N: usize> Future for FirstOk<F, T, E, N>
where
    T: fmt::Debug,
    F: Future<Output = Result<T, E>>,
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

#[cfg(test)]
mod test {
    use super::*;
    use std::future;
    use std::io::{Error, ErrorKind};

    #[test]
    fn all_ok() {
        async_io::block_on(async {
            let res: Result<&str, AggregateError<Error, 2>> =
                [future::ready(Ok("hello")), future::ready(Ok("world"))]
                    .first_ok()
                    .await;
            assert!(res.is_ok());
        })
    }

    #[test]
    fn one_err() {
        async_io::block_on(async {
            let err = Error::new(ErrorKind::Other, "oh no");
            let res: Result<&str, AggregateError<Error, 2>> =
                [future::ready(Ok("hello")), future::ready(Err(err))]
                    .first_ok()
                    .await;
            assert_eq!(res.unwrap(), "hello");
        });
    }

    #[test]
    fn all_err() {
        async_io::block_on(async {
            let err1 = Error::new(ErrorKind::Other, "oops");
            let err2 = Error::new(ErrorKind::Other, "oh no");
            let res: Result<&str, AggregateError<Error, 2>> =
                [future::ready(Err(err1)), future::ready(Err(err2))]
                    .first_ok()
                    .await;
            let errs = res.unwrap_err();
            assert_eq!(errs[0].to_string(), "oops");
            assert_eq!(errs[1].to_string(), "oh no");
        });
    }
}
