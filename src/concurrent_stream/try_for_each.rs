use crate::concurrent_stream::ConsumerState;
use crate::future::FutureGroup;
use crate::private::Try;
use futures_lite::StreamExt;

use super::Consumer;
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::future::Future;
use core::marker::PhantomData;
use core::num::NonZeroUsize;
use core::ops::ControlFlow;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{ready, Context, Poll};

// OK: validated! - all bounds should check out
pub(crate) struct TryForEachConsumer<FutT, T, F, FutB, B>
where
    FutT: Future<Output = T>,
    F: Clone + Fn(T) -> FutB,
    FutB: Future<Output = B>,
    B: Try<Output = ()>,
{
    // NOTE: we can remove the `Arc` here if we're willing to make this struct self-referential
    count: Arc<AtomicUsize>,
    // TODO: remove the `Pin<Box>` from this signature by requiring this struct is pinned
    group: Pin<Box<FutureGroup<TryForEachFut<F, FutT, T, FutB, B>>>>,
    limit: usize,
    err: Option<B::Residual>,
    f: F,
    _phantom: PhantomData<(T, FutB)>,
}

impl<FutT, T, F, FutB, B> TryForEachConsumer<FutT, T, F, FutB, B>
where
    FutT: Future<Output = T>,
    F: Clone + Fn(T) -> FutB,
    FutB: Future<Output = B>,
    B: Try<Output = ()>,
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
impl<FutT, T, F, FutB, B> Consumer<T, FutT> for TryForEachConsumer<FutT, T, F, FutB, B>
where
    FutT: Future<Output = T>,
    F: Clone + Fn(T) -> FutB,
    FutB: Future<Output = B>,
    B: Try<Output = ()>,
{
    type Output = B;

    async fn send(&mut self, future: FutT) -> super::ConsumerState {
        // If we have no space, we're going to provide backpressure until we have space
        while self.count.load(Ordering::Relaxed) >= self.limit {
            match self.group.next().await {
                None => break,
                Some(res) => match res.branch() {
                    ControlFlow::Continue(_) => todo!(),
                    ControlFlow::Break(residual) => {
                        self.err = Some(residual);
                        return ConsumerState::Break;
                    }
                },
            }
        }

        // Space was available! - insert the item for posterity
        self.count.fetch_add(1, Ordering::Relaxed);
        let fut = TryForEachFut::new(self.f.clone(), future, self.count.clone());
        self.group.as_mut().insert_pinned(fut);
        ConsumerState::Continue
    }

    async fn progress(&mut self) -> super::ConsumerState {
        while let Some(res) = self.group.next().await {
            if let ControlFlow::Break(residual) = res.branch() {
                self.err = Some(residual);
                return ConsumerState::Break;
            }
        }
        ConsumerState::Empty
    }

    async fn finish(mut self) -> Self::Output {
        // Return the error if we stopped iteration because of a previous error.
        if let Some(residual) = self.err {
            return B::from_residual(residual);
        }

        // We will no longer receive any additional futures from the
        // underlying stream; wait until all the futures in the group have
        // resolved.
        while let Some(res) = self.group.next().await {
            if let ControlFlow::Break(residual) = res.branch() {
                return B::from_residual(residual);
            }
        }
        B::from_output(())
    }
}

/// Takes a future and maps it to another future via a closure
#[derive(Debug)]
pub struct TryForEachFut<F, FutT, T, FutB, B>
where
    FutT: Future<Output = T>,
    F: Clone + Fn(T) -> FutB,
    FutB: Future<Output = B>,
    B: Try<Output = ()>,
{
    done: bool,
    count: Arc<AtomicUsize>,
    f: F,
    fut_t: Option<FutT>,
    fut_b: Option<FutB>,
}

impl<F, FutT, T, FutB, B> TryForEachFut<F, FutT, T, FutB, B>
where
    FutT: Future<Output = T>,
    F: Clone + Fn(T) -> FutB,
    FutB: Future<Output = B>,
    B: Try<Output = ()>,
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

impl<F, FutT, T, FutB, B> Future for TryForEachFut<F, FutT, T, FutB, B>
where
    FutT: Future<Output = T>,
    F: Clone + Fn(T) -> FutB,
    FutB: Future<Output = B>,
    B: Try<Output = ()>,
{
    type Output = B;

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