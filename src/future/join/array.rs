use super::Join as JoinTrait;
use crate::utils::{self, array_to_manually_drop, PollArray, WakerArray};

use core::array;
use core::fmt;
use core::future::{Future, IntoFuture};
use core::mem::{self, ManuallyDrop, MaybeUninit};
use core::pin::Pin;
use core::task::{Context, Poll};
use std::ops::DerefMut;

use pin_project::{pin_project, pinned_drop};

/// A future which waits for two similarly-typed futures to complete.
///
/// This `struct` is created by the [`join`] method on the [`Join`] trait. See
/// its documentation for more.
///
/// [`join`]: crate::future::Join::join
/// [`Join`]: crate::future::Join
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project(PinnedDrop)]
pub struct Join<Fut, const N: usize>
where
    Fut: Future,
{
    consumed: bool,
    pending: usize,
    items: [MaybeUninit<<Fut as Future>::Output>; N],
    wakers: WakerArray<N>,
    state: PollArray<N>,
    #[pin]
    futures: [ManuallyDrop<Fut>; N],
}

impl<Fut, const N: usize> Join<Fut, N>
where
    Fut: Future,
{
    #[inline]
    pub(crate) fn new(futures: [Fut; N]) -> Self {
        Join {
            consumed: false,
            pending: N,
            items: array::from_fn(|_| MaybeUninit::uninit()),
            wakers: WakerArray::new(),
            state: PollArray::new(),
            futures: array_to_manually_drop(futures),
        }
    }
}

impl<Fut, const N: usize> JoinTrait for [Fut; N]
where
    Fut: IntoFuture,
{
    type Output = [Fut::Output; N];
    type Future = Join<Fut::IntoFuture, N>;

    #[inline]
    fn join(self) -> Self::Future {
        Join::new(self.map(IntoFuture::into_future))
    }
}

impl<Fut, const N: usize> fmt::Debug for Join<Fut, N>
where
    Fut: Future + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.state.iter()).finish()
    }
}

impl<Fut, const N: usize> Future for Join<Fut, N>
where
    Fut: Future,
{
    type Output = [Fut::Output; N];

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        assert!(
            !*this.consumed,
            "Futures must not be polled after completing"
        );

        let mut readiness = this.wakers.readiness().lock().unwrap();
        readiness.set_waker(cx.waker());
        if !readiness.any_ready() {
            // Nothing is ready yet
            return Poll::Pending;
        }

        // Poll all ready futures
        for (i, mut fut) in utils::iter_pin_mut(this.futures.as_mut()).enumerate() {
            if this.state[i].is_pending() && readiness.clear_ready(i) {
                // unlock readiness so we don't deadlock when polling
                drop(readiness);

                // Obtain the intermediate waker.
                let mut cx = Context::from_waker(this.wakers.get(i).unwrap());

                // Poll the future
                // SAFETY: the future's state was "pending", so it's safe to poll
                if let Poll::Ready(value) = unsafe {
                    fut.as_mut()
                        .map_unchecked_mut(|t| t.deref_mut())
                        .poll(&mut cx)
                } {
                    this.items[i] = MaybeUninit::new(value);
                    this.state[i].set_ready();
                    *this.pending -= 1;
                    // SAFETY: the future state has been changed to "ready" which
                    // means we'll no longer poll the future, so it's safe to drop
                    unsafe { ManuallyDrop::drop(fut.get_unchecked_mut()) };
                }

                // Lock readiness so we can use it again
                readiness = this.wakers.readiness().lock().unwrap();
            }
        }

        // Check whether we're all done now or need to keep going.
        if *this.pending == 0 {
            // Mark all data as "consumed" before we take it
            *this.consumed = true;
            for state in this.state.iter_mut() {
                debug_assert!(
                    state.is_ready(),
                    "Future should have reached a `Ready` state"
                );
                state.set_consumed();
            }

            let mut items = array::from_fn(|_| MaybeUninit::uninit());
            mem::swap(this.items, &mut items);

            // SAFETY: we've checked with the state that all of our outputs have been
            // filled, which means we're ready to take the data and assume it's initialized.
            let items = unsafe { utils::array_assume_init(items) };
            Poll::Ready(items)
        } else {
            Poll::Pending
        }
    }
}

/// Drop the already initialized values on cancellation.
#[pinned_drop]
impl<Fut, const N: usize> PinnedDrop for Join<Fut, N>
where
    Fut: Future,
{
    fn drop(self: Pin<&mut Self>) {
        let mut this = self.project();

        // Get the indexes of the initialized output values.
        let indexes = this
            .state
            .iter_mut()
            .enumerate()
            .filter(|(_, state)| state.is_ready())
            .map(|(i, _)| i);

        // Drop each value at the index.
        for i in indexes {
            // SAFETY: we've just filtered down to *only* the initialized values.
            // We can assume they're initialized, and this is where we drop them.
            unsafe { this.items[i].assume_init_drop() };
        }

        // Get the indexes of the pending futures.
        let indexes = this
            .state
            .iter_mut()
            .enumerate()
            .filter(|(_, state)| state.is_pending())
            .map(|(i, _)| i);

        // Drop each future at the index.
        for i in indexes {
            // SAFETY: we've just filtered down to *only* the pending futures,
            // which have not yet been dropped.
            unsafe {
                let futures = this.futures.as_mut().get_unchecked_mut();
                ManuallyDrop::drop(&mut futures[i]);
            };
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::DummyWaker;

    use std::future;
    use std::future::Future;
    use std::sync::Arc;
    use std::task::Context;

    #[test]
    fn smoke() {
        futures_lite::future::block_on(async {
            let fut = [future::ready("hello"), future::ready("world")].join();
            assert_eq!(fut.await, ["hello", "world"]);
        });
    }

    #[test]
    fn debug() {
        let mut fut = [future::ready("hello"), future::ready("world")].join();
        assert_eq!(format!("{:?}", fut), "[Pending, Pending]");
        let mut fut = Pin::new(&mut fut);

        let waker = Arc::new(DummyWaker()).into();
        let mut cx = Context::from_waker(&waker);
        let _ = fut.as_mut().poll(&mut cx);
        assert_eq!(format!("{:?}", fut), "[Consumed, Consumed]");
    }
}
