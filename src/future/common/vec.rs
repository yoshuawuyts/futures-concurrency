use crate::utils::{self, WakerVec};

use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::mem::{self, MaybeUninit};
use std::vec::Vec;

use bitvec::vec::BitVec;
use pin_project::{pin_project, pinned_drop};

pub trait CombinatorBehaviorVec<Fut>
where
    Fut: Future,
{
    type Output;
    type StoredItem;
    fn maybe_return(idx: usize, res: Fut::Output) -> Result<Self::StoredItem, Self::Output>;
    fn when_completed_vec(vec: Vec<Self::StoredItem>) -> Self::Output;
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
pub struct CombinatorVec<Fut, B>
where
    Fut: Future,
    B: CombinatorBehaviorVec<Fut>,
{
    pending: usize,
    items: Vec<MaybeUninit<B::StoredItem>>,
    wakers: WakerVec,
    filled: BitVec,
    awake_list_buffer: Vec<usize>,
    #[pin]
    futures: Vec<Fut>,
}

impl<Fut, B> CombinatorVec<Fut, B>
where
    Fut: Future,
    B: CombinatorBehaviorVec<Fut>,
{
    pub(crate) fn new(futures: Vec<Fut>) -> Self {
        let len = futures.len();
        CombinatorVec {
            pending: len,
            items: std::iter::repeat_with(MaybeUninit::uninit)
                .take(len)
                .collect(),
            wakers: WakerVec::new(len),
            filled: BitVec::repeat(false, len),
            awake_list_buffer: Vec::new(),
            futures,
        }
    }
}

impl<Fut, B> fmt::Debug for CombinatorVec<Fut, B>
where
    Fut: Future + fmt::Debug,
    B: CombinatorBehaviorVec<Fut>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.futures.iter()).finish()
    }
}

impl<Fut, B> Future for CombinatorVec<Fut, B>
where
    Fut: Future,
    B: CombinatorBehaviorVec<Fut>,
{
    type Output = B::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        assert!(
            *this.pending > 0 || this.items.is_empty(),
            "Futures must not be polled after completing"
        );

        {
            let mut awakeness = this.wakers.awakeness();
            awakeness.set_parent_waker(cx.waker());
            this.awake_list_buffer.clone_from(awakeness.awake_list());
            awakeness.clear();
        }

        for idx in this.awake_list_buffer.drain(..) {
            if this.filled[idx] {
                // Woken future is already complete, don't poll it again.
                continue;
            }
            let fut = utils::get_pin_mut_from_vec(this.futures.as_mut(), idx).unwrap();
            let mut cx = Context::from_waker(this.wakers.get(idx).unwrap());
            if let Poll::Ready(value) = fut.poll(&mut cx) {
                match B::maybe_return(idx, value) {
                    Ok(store) => {
                        this.items[idx].write(store);
                        this.filled.set(idx, true);
                        *this.pending -= 1;
                    }
                    Err(ret) => {
                        return Poll::Ready(ret);
                    }
                }
            }
        }

        // Check whether we're all done now or need to keep going.
        if *this.pending == 0 {
            debug_assert!(
                this.filled.iter().all(|filled| *filled),
                "Future should have reached a `Ready` state"
            );
            this.filled.fill(false);

            // SAFETY: we've checked with the state that all of our outputs have been
            // filled, which means we're ready to take the data and assume it's initialized.
            let items = unsafe {
                let items = mem::take(this.items);
                mem::transmute::<Vec<MaybeUninit<B::StoredItem>>, Vec<B::StoredItem>>(items)
            };
            Poll::Ready(B::when_completed_vec(items))
        } else {
            Poll::Pending
        }
    }
}

/// Drop the already initialized values on cancellation.
#[pinned_drop]
impl<Fut, B> PinnedDrop for CombinatorVec<Fut, B>
where
    Fut: Future,
    B: CombinatorBehaviorVec<Fut>,
{
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();

        for (filled, output) in this.filled.iter().zip(this.items.iter_mut()) {
            if *filled {
                unsafe { output.assume_init_drop() }
            }
        }
    }
}
