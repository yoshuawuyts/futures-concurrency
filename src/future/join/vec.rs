use super::Join as JoinTrait;
use crate::utils::{iter_pin_mut_vec, PollState, PollVec};

use core::fmt;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};
use std::mem::{self, MaybeUninit};
use std::vec::Vec;

use pin_project::{pin_project, pinned_drop};

/// Waits for two similarly-typed futures to complete.
///
/// This `struct` is created by the [`join`] method on the [`Join`] trait. See
/// its documentation for more.
///
/// [`join`]: crate::future::Join::join
/// [`Join`]: crate::future::Join
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project(PinnedDrop)]
pub struct Join<Fut>
where
    Fut: Future,
{
    consumed: bool,
    pending: usize,
    items: Vec<MaybeUninit<<Fut as Future>::Output>>,
    state: PollVec,
    #[pin]
    futures: Vec<Fut>,
}

impl<Fut> Join<Fut>
where
    Fut: Future,
{
    pub(crate) fn new(futures: Vec<Fut>) -> Self {
        Join {
            consumed: false,
            pending: futures.len(),
            items: std::iter::repeat_with(MaybeUninit::uninit)
                .take(futures.len())
                .collect(),
            state: PollVec::new(futures.len()),
            futures,
        }
    }
}

impl<Fut> JoinTrait for Vec<Fut>
where
    Fut: IntoFuture,
{
    type Output = Vec<Fut::Output>;
    type Future = Join<Fut::IntoFuture>;

    fn join(self) -> Self::Future {
        Join::new(self.into_iter().map(IntoFuture::into_future).collect())
    }
}

impl<Fut> fmt::Debug for Join<Fut>
where
    Fut: Future + fmt::Debug,
    Fut::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.state.iter()).finish()
    }
}

impl<Fut> Future for Join<Fut>
where
    Fut: Future,
{
    type Output = Vec<Fut::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        assert!(
            !*this.consumed,
            "Futures must not be polled after completing"
        );

        // Poll all futures
        let futures = this.futures.as_mut();
        let states = &mut this.state[..];
        for (i, fut) in iter_pin_mut_vec(futures).enumerate() {
            if states[i].is_pending() {
                if let Poll::Ready(value) = fut.poll(cx) {
                    this.items[i] = MaybeUninit::new(value);
                    states[i] = PollState::Ready;
                    *this.pending -= 1;
                }
            }
        }

        // Check whether we're all done now or need to keep going.
        if *this.pending == 0 {
            // Mark all data as "consumed" before we take it
            *this.consumed = true;
            this.state.iter_mut().for_each(|state| {
                debug_assert!(
                    state.is_ready(),
                    "Future should have reached a `Ready` state"
                );
                *state = PollState::Consumed;
            });

            // SAFETY: we've checked with the state that all of our outputs have been
            // filled, which means we're ready to take the data and assume it's initialized.
            let items = unsafe {
                let items = mem::take(this.items);
                mem::transmute::<_, Vec<Fut::Output>>(items)
            };
            Poll::Ready(items)
        } else {
            Poll::Pending
        }
    }
}

/// Drop the already initialized values on cancellation.
#[pinned_drop]
impl<Fut> PinnedDrop for Join<Fut>
where
    Fut: Future,
{
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();

        // Get the indexes of the initialized values.
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
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::wakers::DummyWaker;

    use std::future;
    use std::future::Future;
    use std::sync::Arc;
    use std::task::Context;

    #[test]
    fn smoke() {
        futures_lite::future::block_on(async {
            let fut = vec![future::ready("hello"), future::ready("world")].join();
            assert_eq!(fut.await, vec!["hello", "world"]);
        });
    }

    #[test]
    fn debug() {
        let mut fut = vec![future::ready("hello"), future::ready("world")].join();
        assert_eq!(format!("{:?}", fut), "[Pending, Pending]");
        let mut fut = Pin::new(&mut fut);

        let waker = Arc::new(DummyWaker()).into();
        let mut cx = Context::from_waker(&waker);
        let _ = fut.as_mut().poll(&mut cx);
        assert_eq!(format!("{:?}", fut), "[Consumed, Consumed]");
    }
}
