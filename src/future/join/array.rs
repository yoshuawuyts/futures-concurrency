use super::Join as JoinTrait;
use crate::utils::{self, WakerArray};

use core::array;
use core::fmt;
use core::future::{Future, IntoFuture};
use core::mem::{self, MaybeUninit};
use core::pin::Pin;
use core::task::{Context, Poll};

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
pub struct Join<Fut, const N: usize>
where
    Fut: Future,
{
    pending: usize,
    items: [MaybeUninit<<Fut as Future>::Output>; N],
    wakers: WakerArray<N>,
    filled: [bool; N],
    awake_list_buffer: [usize; N],
    #[pin]
    futures: [Fut; N],
}

impl<Fut, const N: usize> Join<Fut, N>
where
    Fut: Future,
{
    #[inline]
    pub(crate) fn new(futures: [Fut; N]) -> Self {
        Join {
            pending: N,
            items: array::from_fn(|_| MaybeUninit::uninit()),
            wakers: WakerArray::new(),
            filled: [false; N],
            awake_list_buffer: [0; N],
            futures,
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
    Fut::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.futures.iter()).finish()
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
            N == 0 || *this.pending > 0,
            "Futures must not be polled after completing"
        );

        let num_awake = {
            let mut awakeness = this.wakers.awakeness();
            awakeness.set_parent_waker(cx.waker());
            let awake_list = awakeness.awake_list();
            let num_awake = awake_list.len();
            this.awake_list_buffer[..num_awake].copy_from_slice(awake_list);
            awakeness.clear();
            num_awake
        };

        for &idx in this.awake_list_buffer.iter().take(num_awake) {
            let filled = &mut this.filled[idx];
            if *filled {
                // Woken future is already complete, don't poll it again.
                continue;
            }
            let fut = utils::get_pin_mut(this.futures.as_mut(), idx).unwrap();
            let mut cx = Context::from_waker(this.wakers.get(idx).unwrap());
            if let Poll::Ready(value) = fut.poll(&mut cx) {
                this.items[idx].write(value);
                *filled = true;
                *this.pending -= 1;
            }
        }

        // Check whether we're all done now or need to keep going.
        if *this.pending == 0 {
            for filled in this.filled.iter_mut() {
                debug_assert!(*filled, "Future should have filled items array");
                *filled = false;
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
        let this = self.project();

        for (&filled, output) in this.filled.iter().zip(this.items.iter_mut()) {
            if filled {
                // SAFETY: we've just filtered down to *only* the initialized values.
                // We can assume they're initialized, and this is where we drop them.
                unsafe { output.assume_init_drop() };
            }
        }
    }
}

#[cfg(test)]
mod test {
    use futures_lite::future::yield_now;

    use crate::utils::dummy_waker;

    use super::*;

    use std::cell::RefCell;
    use std::future;
    use std::future::Future;
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
        assert_eq!(fut.filled, [false, false]);
        let mut fut = Pin::new(&mut fut);

        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);
        let _ = fut.as_mut().poll(&mut cx);
        assert_eq!(fut.filled, [false, false]);
    }

    #[test]
    fn poll_order() {
        let polled = RefCell::new(Vec::new());
        async fn record_poll(id: char, times: usize, target: &RefCell<Vec<char>>) {
            for _ in 0..times {
                target.borrow_mut().push(id);
                yield_now().await;
            }
            target.borrow_mut().push(id);
        }
        futures_lite::future::block_on(
            [
                record_poll('a', 0, &polled),
                record_poll('b', 1, &polled),
                record_poll('c', 0, &polled),
            ]
            .join(),
        );
        assert_eq!(&**polled.borrow(), ['a', 'b', 'c', 'b']);

        polled.borrow_mut().clear();
        futures_lite::future::block_on(
            [
                record_poll('a', 2, &polled),
                record_poll('b', 3, &polled),
                record_poll('c', 1, &polled),
                record_poll('d', 0, &polled),
            ]
            .join(),
        );
        assert_eq!(
            &**polled.borrow(),
            ['a', 'b', 'c', 'd', 'a', 'b', 'c', 'a', 'b', 'b']
        );
    }
}
