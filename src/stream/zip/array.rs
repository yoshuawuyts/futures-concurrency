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
    #[pin]
    streams: [S; N],
    items: [MaybeUninit<<S as Stream>::Item>; N],
    wakers: WakerArray<N>,
    filled: [bool; N],
    awake_list_buffer: [usize; N],
    pending: usize,
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
            let mut awakeness = this.wakers.awakeness();
            awakeness.set_parent_waker(cx.waker());
            // pending = usize::MAX is a special value used to communicate that
            // a zipped value has been yielded and everything should be restarted.
            let num_awake = if *this.pending == usize::MAX {
                *this.pending = N;
                *this.awake_list_buffer = array::from_fn(core::convert::identity);
                N
            } else {
                let awake_list = awakeness.awake_list();
                let num_awake = awake_list.len();
                this.awake_list_buffer[..num_awake].copy_from_slice(awake_list);
                num_awake
            };
            awakeness.clear();
            num_awake
        };

        for &idx in this.awake_list_buffer.iter().take(num_awake) {
            let filled = &mut this.filled[idx];
            if *filled {
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

            // Set this so that the wakers get restarted next time.
            *this.pending = usize::MAX;

            let mut items = array::from_fn(|_| MaybeUninit::uninit());
            mem::swap(this.items, &mut items);

            // SAFETY: we've checked with the state that all of our outputs have been
            // filled, which means we're ready to take the data and assume it's initialized.
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
                // SAFETY: we've just filtered down to *only* the initialized values.
                // We can assume they're initialized, and this is where we drop them.
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
