use super::TryJoin as TryJoinTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

#[async_trait::async_trait(?Send)]
impl<Fut, T, E, const N: usize> TryJoinTrait for [Fut; N]
where
    T: std::fmt::Debug,
    Fut: IntoFuture<Output = Result<T, E>>,
    E: fmt::Debug,
{
    type Output = [T; N];
    type Error = E;
    async fn try_join(self) -> Result<Self::Output, Self::Error> {
        TryJoin {
            elems: self.map(|fut| MaybeDone::new(fut.into_future())),
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
pub(super) struct TryJoin<Fut, T, E, const N: usize>
where
    T: fmt::Debug,
    Fut: Future<Output = Result<T, E>>,
{
    elems: [MaybeDone<Fut>; N],
}

impl<Fut, T, E, const N: usize> fmt::Debug for TryJoin<Fut, T, E, N>
where
    Fut: Future<Output = Result<T, E>> + fmt::Debug,
    Fut::Output: fmt::Debug,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Join").field("elems", &self.elems).finish()
    }
}

impl<Fut, T, E, const N: usize> Future for TryJoin<Fut, T, E, N>
where
    T: fmt::Debug,
    Fut: Future<Output = Result<T, E>>,
    E: fmt::Debug,
{
    type Output = Result<[T; N], E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_done = true;

        let this = self.project();

        for elem in this.elems.iter_mut() {
            // SAFETY: we don't ever move the pinned container here; we only pin project
            let mut elem = unsafe { Pin::new_unchecked(elem) };
            if let Poll::Pending = elem.as_mut().poll(cx) {
                all_done = false
            } else if let Some(Err(_)) = elem.as_ref().output() {
                return Poll::Ready(Err(elem.take().unwrap().unwrap_err()));
            }
        }

        if all_done {
            use core::mem::MaybeUninit;

            // Create the result array based on the indices
            let mut out: [MaybeUninit<T>; N] = {
                // inlined version of unstable `MaybeUninit::uninit_array()`
                // TODO: replace with `MaybeUninit::uninit_array()` when it becomes stable
                unsafe { MaybeUninit::<[MaybeUninit<_>; N]>::uninit().assume_init() }
            };

            // NOTE: this clippy attribute can be removed once we can `collect` into `[usize; K]`.
            #[allow(clippy::clippy::needless_range_loop)]
            for (i, el) in this.elems.iter_mut().enumerate() {
                // SAFETY: we don't ever move the pinned container here; we only pin project
                let el = unsafe { Pin::new_unchecked(el) }.take().unwrap().unwrap();
                out[i] = MaybeUninit::new(el);
            }
            let result = unsafe { out.as_ptr().cast::<[T; N]>().read() };
            Poll::Ready(Ok(result))
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
