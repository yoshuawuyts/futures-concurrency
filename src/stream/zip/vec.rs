use super::Zip as ZipTrait;
use crate::stream::IntoStream;
use crate::utils::{self, WakerVec};

use core::fmt;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::mem;

use bitvec::vec::BitVec;
use futures_core::Stream;
use pin_project::{pin_project, pinned_drop};

/// A stream that ‘zips up’ multiple streams into a single stream of pairs.
///
/// This `struct` is created by the [`zip`] method on the [`Zip`] trait. See its
/// documentation for more.
///
/// [`zip`]: trait.Zip.html#method.zip
/// [`Zip`]: trait.Zip.html
#[pin_project(PinnedDrop)]
pub struct Zip<S>
where
    S: Stream,
{
    #[pin]
    streams: Vec<S>,
    items: Vec<MaybeUninit<<S as Stream>::Item>>,
    wakers: WakerVec,
    filled: BitVec,
    awake_list_buffer: Vec<usize>,
    pending: usize,
}

impl<S> Zip<S>
where
    S: Stream,
{
    pub(crate) fn new(streams: Vec<S>) -> Self {
        let len = streams.len();
        Self {
            streams,
            wakers: WakerVec::new(len),
            items: (0..len).map(|_| MaybeUninit::uninit()).collect(),
            filled: BitVec::repeat(false, len),
            awake_list_buffer: Vec::new(),
            pending: len,
        }
    }
}

impl<S> fmt::Debug for Zip<S>
where
    S: Stream + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.streams.iter()).finish()
    }
}

impl<S> Stream for Zip<S>
where
    S: Stream,
{
    type Item = Vec<S::Item>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        let len = this.streams.len();
        {
            let mut awakeness = this.wakers.awakeness();
            awakeness.set_parent_waker(cx.waker());
            if *this.pending == usize::MAX {
                *this.pending = len;
                this.awake_list_buffer.clear();
                this.awake_list_buffer.extend(0..len);
            } else {
                this.awake_list_buffer.clone_from(awakeness.awake_list());
            }
            awakeness.clear();
        }

        for idx in this.awake_list_buffer.drain(..) {
            let mut filled = this.filled.get_mut(idx).unwrap();
            if *filled {
                continue;
            }
            let stream = utils::get_pin_mut_from_vec(this.streams.as_mut(), idx).unwrap();
            let mut cx = Context::from_waker(this.wakers.get(idx).unwrap());
            match stream.poll_next(&mut cx) {
                Poll::Ready(Some(value)) => {
                    this.items[idx].write(value);
                    filled.set(true);
                    *this.pending -= 1;
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => {}
            }
        }

        if *this.pending == 0 {
            for mut filled in this.filled.iter_mut() {
                debug_assert!(*filled, "The items array should have been filled");
                filled.set(false);
            }
            *this.pending = usize::MAX;

            let mut output = (0..len).map(|_| MaybeUninit::uninit()).collect();
            mem::swap(this.items, &mut output);
            let output = unsafe { vec_assume_init(output) };
            Poll::Ready(Some(output))
        } else {
            Poll::Pending
        }
    }
}

/// Drop the already initialized values on cancellation.
#[pinned_drop]
impl<S> PinnedDrop for Zip<S>
where
    S: Stream,
{
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();

        for (filled, output) in this.filled.iter().zip(this.items.iter_mut()) {
            if *filled {
                // SAFETY: we've just filtered down to *only* the initialized values.
                // We can assume they're initialized, and this is where we drop them.
                unsafe { output.assume_init_drop() };
            }
        }
    }
}

impl<S> ZipTrait for Vec<S>
where
    S: IntoStream,
{
    type Item = <Zip<S::IntoStream> as Stream>::Item;
    type Stream = Zip<S::IntoStream>;

    fn zip(self) -> Self::Stream {
        Zip::new(self.into_iter().map(|i| i.into_stream()).collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::stream::Zip;
    use futures_lite::future::block_on;
    use futures_lite::prelude::*;
    use futures_lite::stream;

    #[test]
    fn zip_array_3() {
        block_on(async {
            let a = stream::repeat(1).take(2);
            let b = stream::repeat(2).take(2);
            let c = stream::repeat(3).take(2);
            let mut s = vec![a, b, c].zip();

            assert_eq!(s.next().await, Some(vec![1, 2, 3]));
            assert_eq!(s.next().await, Some(vec![1, 2, 3]));
            assert_eq!(s.next().await, None);
        })
    }
}

// Inlined version of the unstable `MaybeUninit::array_assume_init` feature.
// FIXME: replace with `utils::array_assume_init`
unsafe fn vec_assume_init<T>(vec: Vec<MaybeUninit<T>>) -> Vec<T> {
    // SAFETY:
    // * The caller guarantees that all elements of the vec are initialized
    // * `MaybeUninit<T>` and T are guaranteed to have the same layout
    // * `MaybeUninit` does not drop, so there are no double-frees
    // And thus the conversion is safe
    let ret = unsafe { (&vec as *const _ as *const Vec<T>).read() };
    mem::forget(vec);
    ret
}
