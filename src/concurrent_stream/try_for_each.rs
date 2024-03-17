use crate::concurrent_stream::ConsumerState;
use crate::future::FutureGroup;
use futures_lite::StreamExt;

use super::Consumer;
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::future::Future;
use core::marker::PhantomData;
use core::num::NonZeroUsize;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{ready, Context, Poll};

// OK: validated! - all bounds should check out
pub(crate) struct TryForEachConsumer<FutT, T, F, FutB, E>
where
    FutT: Future<Output = T>,
    F: Fn(T) -> FutB,
    FutB: Future<Output = Result<(), E>>,
{
    // NOTE: we can remove the `Arc` here if we're willing to make this struct self-referential
    count: Arc<AtomicUsize>,
    // TODO: remove the `Pin<Box>` from this signature by requiring this struct is pinned
    group: Pin<Box<FutureGroup<TryForEachFut<F, FutT, T, FutB, E>>>>,
    limit: usize,
    err: Option<E>,
    f: F,
    _phantom: PhantomData<(T, FutB)>,
}

impl<A, T, F, B, E> TryForEachConsumer<A, T, F, B, E>
where
    A: Future<Output = T>,
    F: Fn(T) -> B,
    B: Future<Output = Result<(), E>>,
{
    pub(crate) fn new(limit: Option<NonZeroUsize>, f: F) -> Self {
        let limit = match limit {
            Some(n) => n.get(),
            None => usize::MAX,
        };
        Self {
            limit,
            f,
            err: None,
            count: Arc::new(AtomicUsize::new(0)),
            group: Box::pin(FutureGroup::new()),
            _phantom: PhantomData,
        }
    }
}

// OK: validated! - we push types `B` into the next consumer
impl<FutT, T, F, B, E> Consumer<T, FutT> for TryForEachConsumer<FutT, T, F, B, E>
where
    FutT: Future<Output = T>,
    F: Fn(T) -> B,
    F: Clone,
    B: Future<Output = Result<(), E>>,
{
    type Output = Result<(), E>;

    async fn send(&mut self, future: FutT) -> super::ConsumerState {
        // If we have no space, we're going to provide backpressure until we have space
        while self.count.load(Ordering::Relaxed) >= self.limit {
            match self.group.next().await {
                Some(Ok(_)) => continue,
                Some(Err(err)) => {
                    self.err = Some(err);
                    return ConsumerState::Break;
                }
                None => break,
            };
        }

        // Space was available! - insert the item for posterity
        self.count.fetch_add(1, Ordering::Relaxed);
        let fut = TryForEachFut::new(self.f.clone(), future, self.count.clone());
        self.group.as_mut().insert_pinned(fut);
        ConsumerState::Continue
    }

    async fn progress(&mut self) -> super::ConsumerState {
        while let Some(res) = self.group.next().await {
            if let Err(err) = res {
                self.err = Some(err);
                return ConsumerState::Break;
            }
        }
        ConsumerState::Empty
    }

    async fn finish(mut self) -> Self::Output {
        // Return the error if we stopped iteration because of a previous error.
        if let Some(err) = self.err {
            return Err(err);
        }

        // We will no longer receive any additional futures from the
        // underlying stream; wait until all the futures in the group have
        // resolved.
        while let Some(res) = self.group.next().await {
            res?;
        }
        Ok(())
    }
}

/// Takes a future and maps it to another future via a closure
#[derive(Debug)]
pub struct TryForEachFut<F, FutT, T, FutB, E>
where
    FutT: Future<Output = T>,
    F: Fn(T) -> FutB,
    FutB: Future<Output = Result<(), E>>,
{
    done: bool,
    count: Arc<AtomicUsize>,
    f: F,
    fut_t: Option<FutT>,
    fut_b: Option<FutB>,
}

impl<F, FutT, T, FutB, E> TryForEachFut<F, FutT, T, FutB, E>
where
    FutT: Future<Output = T>,
    F: Fn(T) -> FutB,
    FutB: Future<Output = Result<(), E>>,
{
    fn new(f: F, fut_t: FutT, count: Arc<AtomicUsize>) -> Self {
        Self {
            done: false,
            count,
            f,
            fut_t: Some(fut_t),
            fut_b: None,
        }
    }
}

impl<F, FutT, T, FutB, E> Future for TryForEachFut<F, FutT, T, FutB, E>
where
    FutT: Future<Output = T>,
    F: Fn(T) -> FutB,
    FutB: Future<Output = Result<(), E>>,
{
    type Output = Result<(), E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: we need to access the inner future's fields to project them
        let this = unsafe { self.get_unchecked_mut() };
        if this.done {
            panic!("future has already been polled to completion once");
        }

        // Poll forward the future containing the value of `T`
        if let Some(fut) = this.fut_t.as_mut() {
            // SAFETY: we're pin projecting here
            let t = ready!(unsafe { Pin::new_unchecked(fut) }.poll(cx));
            let fut_b = (this.f)(t);
            this.fut_t = None;
            this.fut_b = Some(fut_b);
        }

        // Poll forward the future returned by the closure
        if let Some(fut) = this.fut_b.as_mut() {
            // SAFETY: we're pin projecting here
            let item = ready!(unsafe { Pin::new_unchecked(fut) }.poll(cx));
            this.count.fetch_sub(1, Ordering::Relaxed);
            this.done = true;
            return Poll::Ready(item);
        }

        unreachable!("neither future `a` nor future `b` were ready");
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::*;
    use futures_lite::stream;
    use std::{io, sync::Arc};

    #[test]
    fn concurrency_one() {
        futures_lite::future::block_on(async {
            let count = Arc::new(AtomicUsize::new(0));
            stream::repeat(1)
                .take(2)
                .co()
                .limit(NonZeroUsize::new(1))
                .try_for_each(|n| {
                    let count = count.clone();
                    async move {
                        count.fetch_add(n, Ordering::Relaxed);
                        std::io::Result::Ok(())
                    }
                })
                .await
                .unwrap();

            assert_eq!(count.load(Ordering::Relaxed), 2);
        });
    }

    #[test]
    fn concurrency_three() {
        futures_lite::future::block_on(async {
            let count = Arc::new(AtomicUsize::new(0));
            stream::repeat(1)
                .take(10)
                .co()
                .limit(NonZeroUsize::new(3))
                .try_for_each(|n| {
                    let count = count.clone();
                    async move {
                        count.fetch_add(n, Ordering::Relaxed);
                        std::io::Result::Ok(())
                    }
                })
                .await
                .unwrap();

            assert_eq!(count.load(Ordering::Relaxed), 10);
        });
    }

    #[test]
    fn short_circuits() {
        futures_lite::future::block_on(async {
            let count = Arc::new(AtomicUsize::new(0));
            let output = stream::repeat(10)
                .take(2)
                .co()
                .limit(NonZeroUsize::new(1))
                .try_for_each(|n| {
                    let count = count.clone();
                    async move {
                        count.fetch_add(n, Ordering::SeqCst);
                        std::io::Result::Err(io::ErrorKind::Other.into())
                    }
                })
                .await;

            assert!(output.is_err());
        });
    }
}

// /// We hide the `Try` trait in a private module, as it's only meant to be a
// /// stable clone of the standard library's `Try` trait, as yet unstable.
// // NOTE: copied from `rayon`
// mod private {
//     use std::convert::Infallible;
//     use std::ops::ControlFlow::{self, Break, Continue};
//     use std::task::Poll;

//     /// Clone of `std::ops::Try`.
//     ///
//     /// Implementing this trait is not permitted outside of `futures_concurrency`.
//     pub trait Try {
//         private_decl! {}

//         type Output;
//         type Residual;

//         fn from_output(output: Self::Output) -> Self;

//         fn from_residual(residual: Self::Residual) -> Self;

//         fn branch(self) -> ControlFlow<Self::Residual, Self::Output>;
//     }

//     impl<B, C> Try for ControlFlow<B, C> {
//         private_impl! {}

//         type Output = C;
//         type Residual = ControlFlow<B, Infallible>;

//         fn from_output(output: Self::Output) -> Self {
//             Continue(output)
//         }

//         fn from_residual(residual: Self::Residual) -> Self {
//             match residual {
//                 Break(b) => Break(b),
//                 Continue(_) => unreachable!(),
//             }
//         }

//         fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
//             match self {
//                 Continue(c) => Continue(c),
//                 Break(b) => Break(Break(b)),
//             }
//         }
//     }

//     impl<T> Try for Option<T> {
//         private_impl! {}

//         type Output = T;
//         type Residual = Option<Infallible>;

//         fn from_output(output: Self::Output) -> Self {
//             Some(output)
//         }

//         fn from_residual(residual: Self::Residual) -> Self {
//             match residual {
//                 None => None,
//                 Some(_) => unreachable!(),
//             }
//         }

//         fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
//             match self {
//                 Some(c) => Continue(c),
//                 None => Break(None),
//             }
//         }
//     }

//     impl<T, E> Try for Result<T, E> {
//         private_impl! {}

//         type Output = T;
//         type Residual = Result<Infallible, E>;

//         fn from_output(output: Self::Output) -> Self {
//             Ok(output)
//         }

//         fn from_residual(residual: Self::Residual) -> Self {
//             match residual {
//                 Err(e) => Err(e),
//                 Ok(_) => unreachable!(),
//             }
//         }

//         fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
//             match self {
//                 Ok(c) => Continue(c),
//                 Err(e) => Break(Err(e)),
//             }
//         }
//     }

//     impl<T, E> Try for Poll<Result<T, E>> {
//         private_impl! {}

//         type Output = Poll<T>;
//         type Residual = Result<Infallible, E>;

//         fn from_output(output: Self::Output) -> Self {
//             output.map(Ok)
//         }

//         fn from_residual(residual: Self::Residual) -> Self {
//             match residual {
//                 Err(e) => Poll::Ready(Err(e)),
//                 Ok(_) => unreachable!(),
//             }
//         }

//         fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
//             match self {
//                 Poll::Pending => Continue(Poll::Pending),
//                 Poll::Ready(Ok(c)) => Continue(Poll::Ready(c)),
//                 Poll::Ready(Err(e)) => Break(Err(e)),
//             }
//         }
//     }

//     impl<T, E> Try for Poll<Option<Result<T, E>>> {
//         private_impl! {}

//         type Output = Poll<Option<T>>;
//         type Residual = Result<Infallible, E>;

//         fn from_output(output: Self::Output) -> Self {
//             match output {
//                 Poll::Ready(o) => Poll::Ready(o.map(Ok)),
//                 Poll::Pending => Poll::Pending,
//             }
//         }

//         fn from_residual(residual: Self::Residual) -> Self {
//             match residual {
//                 Err(e) => Poll::Ready(Some(Err(e))),
//                 Ok(_) => unreachable!(),
//             }
//         }

//         fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
//             match self {
//                 Poll::Pending => Continue(Poll::Pending),
//                 Poll::Ready(None) => Continue(Poll::Ready(None)),
//                 Poll::Ready(Some(Ok(c))) => Continue(Poll::Ready(Some(c))),
//                 Poll::Ready(Some(Err(e))) => Break(Err(e)),
//             }
//         }
//     }

//     #[allow(missing_debug_implementations)]
//     pub struct PrivateMarker;
//     macro_rules! private_impl {
//         () => {
//             fn __futures_concurrency_private__(&self) -> crate::private::PrivateMarker {
//                 crate::private::PrivateMarker
//             }
//         };
//     }

//     macro_rules! private_decl {
//         () => {
//             /// This trait is private; this method exists to make it
//             /// impossible to implement outside the crate.
//             #[doc(hidden)]
//             fn __futures_concurrency_private__(&self) -> crate::private::PrivateMarker;
//         };
//     }
// }
