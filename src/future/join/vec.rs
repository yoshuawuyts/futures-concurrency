use super::Join as JoinTrait;
use crate::utils::{iter_pin_mut_vec, Metadata};

use core::fmt;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};
use std::mem::{self, MaybeUninit};
use std::vec::Vec;

use pin_project::{pin_project, pinned_drop};

/// Waits for two similarly-typed futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the
/// futures once both complete.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project(PinnedDrop)]
pub struct Join<Fut>
where
    Fut: Future,
{
    #[pin]
    futures: Vec<Fut>,
    items: Vec<MaybeUninit<<Fut as Future>::Output>>,
    metadata: Vec<Metadata>,
}

impl<Fut> Join<Fut>
where
    Fut: Future,
{
    pub(crate) fn new(futures: Vec<Fut>) -> Self {
        Join {
            items: std::iter::repeat_with(|| MaybeUninit::uninit())
                .take(futures.len())
                .collect(),
            metadata: std::iter::successors(Some(0), |prev| Some(prev + 1))
                .take(futures.len())
                .map(Metadata::new)
                .collect(),
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
        // TODO: fix debug output
        f.debug_struct("Join").finish()
    }
}

impl<Fut> Future for Join<Fut>
where
    Fut: Future,
{
    type Output = Vec<Fut::Output>;

    // SAFETY: see https://github.com/rust-lang/rust/issues/104108,
    // projecting through slices is fine now, but it's not yet guaranteed to
    // work. We need to guarantee structural pinning works as expected for it to
    // be provably sound.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        // Poll all futures
        let futures = this.futures.as_mut();
        for (i, fut) in iter_pin_mut_vec(futures).enumerate() {
            if this.metadata[i].is_done() {
                continue;
            }

            if let Poll::Ready(value) = fut.poll(cx) {
                this.items[i] = MaybeUninit::new(value);
                this.metadata[i].set_done();
            }
        }

        // Check whether we're all done now or need to keep going.
        if this.metadata.iter().all(|meta| meta.is_done()) {
            // Mark all data as "taken" before we actually take it.
            this.metadata.iter_mut().for_each(|meta| meta.set_taken());

            // SAFETY: we've checked with the metadata that all of our outputs have been
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
            .metadata
            .iter_mut()
            .filter(|meta| meta.is_done())
            .map(|meta| meta.index());

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
    use std::future;

    // NOTE: we should probably poll in random order.
    #[test]
    fn no_fairness() {
        futures_lite::future::block_on(async {
            let res = vec![future::ready("hello"), future::ready("world")]
                .join()
                .await;
            assert_eq!(res, vec!["hello", "world"]);
        });
    }
}
