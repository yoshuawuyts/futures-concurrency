use super::Zip as ZipTrait;
use crate::stream::IntoStream;
use crate::utils::{self, WakerArray};

use core::array;
use core::fmt;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::mem;

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
pub struct Zip<S, const N: usize>
where
    S: Stream,
{
    // Number of substreams that we're waiting for.
    // MAGIC VALUE: pending == usize::MAX is used to signal that
    // we just yielded a zipped value and every stream should be woken up again.
    //
    // pending value goes like
    // N,N-1,...,1,0,
    // <zipped item yielded>,usize::MAX,
    // N,N-1,...,1,0,
    // <zipped item yielded>,usize::MAX,...
    pending: usize,
    wakers: WakerArray<N>,
    /// The stored output from each substream.
    items: [MaybeUninit<<S as Stream>::Item>; N],
    /// Whether each item in self.items is initialized.
    /// Invariant: self.filled.count_falses() == self.pending
    /// EXCEPT when self.pending==usize::MAX, self.filled must be all false.
    filled: [bool; N],
    /// A temporary buffer for indices that have woken.
    /// The data here don't have to persist between each `poll_next`.
    awake_list_buffer: [usize; N],
    #[pin]
    streams: [S; N],
    /// Streams should not be polled after complete.
    /// In debug, we panic to the user.
    /// In release, we might sleep or poll substreams after completion.
    #[cfg(debug_assertions)]
    done: bool,
}

impl<S, const N: usize> Zip<S, N>
where
    S: Stream,
{
    pub(crate) fn new(streams: [S; N]) -> Self {
        Self {
            streams,
            items: array::from_fn(|_| MaybeUninit::uninit()),
            filled: [false; N],
            wakers: WakerArray::new(),
            // TODO: this is a temporary buffer so it can be MaybeUninit.
            awake_list_buffer: [0; N],
            pending: N,
            #[cfg(debug_assertions)]
            done: false,
        }
    }
}

impl<S, const N: usize> fmt::Debug for Zip<S, N>
where
    S: Stream + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.streams.iter()).finish()
    }
}

impl<S, const N: usize> Stream for Zip<S, N>
where
    S: Stream,
{
    type Item = [S::Item; N];

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        #[cfg(debug_assertions)]
        assert!(!*this.done, "Stream should not be polled after completing");

        let num_awake = {
            // Lock the awakeness Mutex.
            let mut awakeness = this.wakers.awakeness();
            awakeness.set_parent_waker(cx.waker());

            let num_awake = if *this.pending == usize::MAX {
                // pending = usize::MAX is a special value used to communicate that
                // a zipped value has been yielded and everything should be restarted.
                *this.pending = N;
                // Fill the awake_list_buffer with 0..N.
                *this.awake_list_buffer = array::from_fn(core::convert::identity);
                N
            } else {
                // Copy the awake list out of the Mutex.
                let awake_list = awakeness.awake_list();
                let num_awake = awake_list.len();
                this.awake_list_buffer[..num_awake].copy_from_slice(awake_list);
                num_awake
            };
            // Clear the list in the Mutex.
            awakeness.clear();
            num_awake
        };

        // Iterate over the awake list.
        for &idx in this.awake_list_buffer.iter().take(num_awake) {
            let filled = &mut this.filled[idx];
            if *filled {
                // Woken substream has already yielded.
                continue;
            }
            let stream = utils::get_pin_mut(this.streams.as_mut(), idx).unwrap();
            let mut cx = Context::from_waker(this.wakers.get(idx).unwrap());
            match stream.poll_next(&mut cx) {
                Poll::Ready(Some(value)) => {
                    this.items[idx].write(value);
                    *filled = true;
                    *this.pending -= 1;
                }
                Poll::Ready(None) => {
                    // If one substream ends, the entire Zip ends.

                    #[cfg(debug_assertions)]
                    {
                        *this.done = true;
                    }

                    return Poll::Ready(None);
                }
                Poll::Pending => {}
            }
        }

        if *this.pending == 0 {
            debug_assert!(
                this.filled.iter().all(|&filled| filled),
                "The items array should have been filled"
            );
            this.filled.fill(false);

            // Set this to the magic value so that the wakers get restarted next time.
            *this.pending = usize::MAX;

            let mut items = array::from_fn(|_| MaybeUninit::uninit());
            mem::swap(this.items, &mut items);

            // SAFETY: this.pending is only decremented when an item slot is filled.
            // pending reaching 0 means the entire items array is filled.
            let items = unsafe { utils::array_assume_init(items) };
            Poll::Ready(Some(items))
        } else {
            Poll::Pending
        }
    }
}

/// Drop the already initialized values on cancellation.
#[pinned_drop]
impl<S, const N: usize> PinnedDrop for Zip<S, N>
where
    S: Stream,
{
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();

        for (&filled, output) in this.filled.iter().zip(this.items.iter_mut()) {
            if filled {
                // SAFETY: when filled is true the item must be initialized.
                unsafe { output.assume_init_drop() };
            }
        }
    }
}

impl<S, const N: usize> ZipTrait for [S; N]
where
    S: IntoStream,
{
    type Item = <Zip<S::IntoStream, N> as Stream>::Item;
    type Stream = Zip<S::IntoStream, N>;

    fn zip(self) -> Self::Stream {
        Zip::new(self.map(|i| i.into_stream()))
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
            let mut s = Zip::zip([a, b, c]);

            assert_eq!(s.next().await, Some([1, 2, 3]));
            assert_eq!(s.next().await, Some([1, 2, 3]));
            assert_eq!(s.next().await, None);
        })
    }
}
