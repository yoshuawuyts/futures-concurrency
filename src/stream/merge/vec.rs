use super::Merge as MergeTrait;
use crate::stream::IntoStream;
use crate::utils::{self, WakerVec};

use bitvec::vec::BitVec;
use core::fmt;
use futures_core::Stream;
use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A stream that merges multiple streams into a single stream.
///
/// This `struct` is created by the [`merge`] method on the [`Merge`] trait. See its
/// documentation for more.
///
/// [`merge`]: trait.Merge.html#method.merge
/// [`Merge`]: trait.Merge.html
#[pin_project::pin_project]
pub struct Merge<S>
where
    S: Stream,
{
    #[pin]
    streams: Vec<S>,
    wakers: WakerVec,
    pending: usize,
    consumed: BitVec,
    awake_set: BitVec,
    awake_list: VecDeque<usize>,
}

impl<S> Merge<S>
where
    S: Stream,
{
    pub(crate) fn new(streams: Vec<S>) -> Self {
        let len = streams.len();
        Self {
            streams,
            wakers: WakerVec::new(len),
            pending: len,
            consumed: BitVec::repeat(false, len),
            awake_set: BitVec::repeat(false, len),
            awake_list: VecDeque::with_capacity(len),
        }
    }
}

impl<S> fmt::Debug for Merge<S>
where
    S: Stream + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.streams.iter()).finish()
    }
}

impl<S> Stream for Merge<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        {
            let mut awakeness = this.wakers.awakeness();
            awakeness.set_parent_waker(cx.waker());
            let awake_list = awakeness.awake_list();
            let awake_set = &mut *this.awake_set;
            let consumed = &mut *this.consumed;
            this.awake_list.extend(awake_list.iter().filter_map(|&idx| {
                (!awake_set.replace(idx, true) && !consumed[idx]).then_some(idx)
            }));
            awakeness.clear();
        }

        while let Some(idx) = this.awake_list.pop_front() {
            if this.consumed[idx] {
                continue;
            }
            this.awake_set.set(idx, false);
            let waker = this.wakers.get(idx).unwrap();
            let mut cx = Context::from_waker(waker);
            let stream = utils::get_pin_mut_from_vec(this.streams.as_mut(), idx).unwrap();
            match stream.poll_next(&mut cx) {
                Poll::Ready(Some(item)) => {
                    waker.wake_by_ref();
                    return Poll::Ready(Some(item));
                }
                Poll::Ready(None) => {
                    *this.pending -= 1;
                    this.consumed.set(idx, true);
                }
                Poll::Pending => {}
            }
        }
        if *this.pending == 0 {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

impl<S> MergeTrait for Vec<S>
where
    S: IntoStream,
{
    type Item = <Merge<S::IntoStream> as Stream>::Item;
    type Stream = Merge<S::IntoStream>;

    fn merge(self) -> Self::Stream {
        Merge::new(self.into_iter().map(|i| i.into_stream()).collect())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::utils::channel::local_channel;
    use futures::executor::LocalPool;
    use futures::task::LocalSpawnExt;
    use futures_lite::future::block_on;
    use futures_lite::prelude::*;
    use futures_lite::stream;

    use crate::future::join::Join;

    #[test]
    fn merge_vec_4() {
        block_on(async {
            let a = stream::once(1);
            let b = stream::once(2);
            let c = stream::once(3);
            let d = stream::once(4);
            let mut s = vec![a, b, c, d].merge();

            let mut counter = 0;
            while let Some(n) = s.next().await {
                counter += n;
            }
            assert_eq!(counter, 10);
        })
    }

    #[test]
    fn merge_vec_2x2() {
        block_on(async {
            let a = stream::repeat(1).take(2);
            let b = stream::repeat(2).take(2);
            let mut s = vec![a, b].merge();

            let mut counter = 0;
            while let Some(n) = s.next().await {
                counter += n;
            }
            assert_eq!(counter, 6);
        })
    }

    /// This test case uses channels so we'll have streams that return Pending from time to time.
    ///
    /// The purpose of this test is to make sure we have the waking logic working.
    #[test]
    fn merge_channels() {
        let mut pool = LocalPool::new();

        let done = Rc::new(RefCell::new(false));
        let done2 = done.clone();

        pool.spawner()
            .spawn_local(async move {
                let (send1, receive1) = local_channel();
                let (send2, receive2) = local_channel();
                let (send3, receive3) = local_channel();

                let (count, ()) = (
                    async {
                        vec![receive1, receive2, receive3]
                            .merge()
                            .fold(0, |a, b| a + b)
                            .await
                    },
                    async {
                        for i in 1..=4 {
                            send1.send(i);
                            send2.send(i);
                            send3.send(i);
                        }
                        drop(send1);
                        drop(send2);
                        drop(send3);
                    },
                )
                    .join()
                    .await;

                assert_eq!(count, 30);

                *done2.borrow_mut() = true;
            })
            .unwrap();

        while !*done.borrow() {
            pool.run_until_stalled()
        }
    }
}
