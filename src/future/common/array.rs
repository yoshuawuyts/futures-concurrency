use crate::utils::{self, WakerArray};

use core::array;
use core::fmt;
use core::future::Future;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::{pin_project, pinned_drop};

pub trait CombinatorBehaviorArray<Fut, const N: usize>
where
    Fut: Future,
{
    type Output;
    type StoredItem;
    fn maybe_return(idx: usize, res: Fut::Output) -> Result<Self::StoredItem, Self::Output>;
    fn when_completed_arr(arr: [Self::StoredItem; N]) -> Self::Output;
}

/// Waits for two similarly-typed futures to complete.
///
/// This `struct` is created by the [`join`] method on the [`Join`] trait. See
/// its documentation for more.
///
/// [`join`]: crate::future::Join::join
/// [`Join`]: crate::future::Join
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project(PinnedDrop)]
pub struct CombinatorArray<Fut, B, const N: usize>
where
    Fut: Future,
    B: CombinatorBehaviorArray<Fut, N>,
{
    behavior: PhantomData<B>,
    pending: usize,
    items: [MaybeUninit<B::StoredItem>; N],
    wakers: WakerArray<N>,
    filled: [bool; N],
    awake_list_buffer: [usize; N],
    #[pin]
    futures: [Fut; N],
}

impl<Fut, B, const N: usize> CombinatorArray<Fut, B, N>
where
    Fut: Future,
    B: CombinatorBehaviorArray<Fut, N>,
{
    #[inline]
    pub(crate) fn new(futures: [Fut; N]) -> Self {
        CombinatorArray {
            behavior: PhantomData,
            pending: N,
            items: array::from_fn(|_| MaybeUninit::uninit()),
            wakers: WakerArray::new(),
            filled: [false; N],
            awake_list_buffer: [0; N],
            futures,
        }
    }
}

impl<Fut, B, const N: usize> fmt::Debug for CombinatorArray<Fut, B, N>
where
    Fut: Future + fmt::Debug,
    B: CombinatorBehaviorArray<Fut, N>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.futures.iter()).finish()
    }
}

impl<Fut, B, const N: usize> Future for CombinatorArray<Fut, B, N>
where
    Fut: Future,
    B: CombinatorBehaviorArray<Fut, N>,
{
    type Output = B::Output;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        assert!(
            *this.pending > 0,
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
                match B::maybe_return(idx, value) {
                    Ok(store) => {
                        this.items[idx].write(store);
                        *filled = true;
                        *this.pending -= 1;
                    }
                    Err(ret) => return Poll::Ready(ret),
                }
            }
        }

        // Check whether we're all done now or need to keep going.
        if *this.pending == 0 {
            debug_assert!(
                this.filled.iter().all(|&filled| filled),
                "Future should have filled items array"
            );
            this.filled.fill(false);

            let mut items = array::from_fn(|_| MaybeUninit::uninit());
            core::mem::swap(this.items, &mut items);

            // SAFETY: we've checked with the state that all of our outputs have been
            // filled, which means we're ready to take the data and assume it's initialized.
            let items = unsafe { utils::array_assume_init(items) };

            Poll::Ready(B::when_completed_arr(items))
        } else {
            Poll::Pending
        }
    }
}

/// Drop the already initialized values on cancellation.
#[pinned_drop]
impl<Fut, B, const N: usize> PinnedDrop for CombinatorArray<Fut, B, N>
where
    Fut: Future,
    B: CombinatorBehaviorArray<Fut, N>,
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
