use super::Merge as MergeTrait;
use crate::stream::IntoStream;
use crate::utils::{self, ArrayDequeue, PollState, WakerArray};

use core::fmt;
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_core::Stream;

/// A stream that merges multiple streams into a single stream.
///
/// This `struct` is created by the [`merge`] method on the [`Merge`] trait. See its
/// documentation for more.
///
/// [`merge`]: trait.Merge.html#method.merge
/// [`Merge`]: trait.Merge.html
#[pin_project::pin_project]
pub struct Merge<S, const N: usize>
where
    S: Stream,
{
    #[pin]
    streams: [S; N],
    wakers: WakerArray<N>,
    /// Number of substreams that haven't completed.
    pending: usize,
    /// The states of the N streams.
    /// Pending = stream is sleeping
    /// Ready = stream is awake
    /// Consumed = stream is complete
    state: [PollState; N],
    /// List of awoken streams.
    awake_list: ArrayDequeue<usize, N>,
    /// Streams should not be polled after complete.
    /// In debug, we panic to the user.
    /// In release, we might sleep or poll substreams after completion.
    #[cfg(debug_assertions)]
    done: bool,
}

impl<S, const N: usize> Merge<S, N>
where
    S: Stream,
{
    pub(crate) fn new(streams: [S; N]) -> Self {
        Self {
            streams,
            wakers: WakerArray::new(),
            pending: N,
            // Start with every substream awake.
            state: [PollState::Ready; N],
            // Start with indices of every substream since they're all awake.
            awake_list: ArrayDequeue::new(core::array::from_fn(core::convert::identity), N),
            #[cfg(debug_assertions)]
            done: false,
        }
    }
}

impl<S, const N: usize> fmt::Debug for Merge<S, N>
where
    S: Stream + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.streams.iter()).finish()
    }
}

impl<S, const N: usize> Stream for Merge<S, N>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        #[cfg(debug_assertions)]
        assert!(!*this.done, "Stream should not be polled after completing");

        {
            // Lock the readiness Mutex.
            let mut readiness = this.wakers.readiness();
            readiness.set_parent_waker(cx.waker());

            // Copy over the indices of awake substreams.
            let awake_list = readiness.awake_list();
            let states = &mut *this.state;
            this.awake_list.extend(awake_list.iter().filter_map(|&idx| {
                // Only add to our list if the substream is actually pending.
                // Our awake list will never contain duplicate indices.
                let state = &mut states[idx];
                match state {
                    PollState::Pending => {
                        // Set the state to awake.
                        *state = PollState::Ready;
                        Some(idx)
                    }
                    _ => None,
                }
            }));

            // Clear the list in the Mutex.
            readiness.clear();

            // The Mutex should be unlocked here.
        }

        for idx in this.awake_list.drain() {
            let state = &mut this.state[idx];
            // At this point state must be PollState::Ready (substream is awake).

            let waker = this.wakers.get(idx).unwrap();
            let mut cx = Context::from_waker(waker);
            let stream = utils::get_pin_mut(this.streams.as_mut(), idx).unwrap();
            match stream.poll_next(&mut cx) {
                Poll::Ready(Some(item)) => {
                    // Queue the substream to be polled again next time.
                    // Todo: figure out how to do this without locking the Mutex.
                    // We can do `this.awake_list.push_back(idx)`
                    // but that will cause this substream to be scheduled before the others that have woken
                    // between the indices-copying and now, leading to unfairness.
                    waker.wake_by_ref();
                    *state = PollState::Pending;

                    return Poll::Ready(Some(item));
                }
                Poll::Ready(None) => {
                    *this.pending -= 1;
                    *state = PollState::Consumed;
                }
                Poll::Pending => {
                    *state = PollState::Pending;
                }
            }
        }

        if *this.pending == 0 {
            #[cfg(debug_assertions)]
            {
                *this.done = true;
            }
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

impl<S, const N: usize> MergeTrait for [S; N]
where
    S: IntoStream,
{
    type Item = <Merge<S::IntoStream, N> as Stream>::Item;
    type Stream = Merge<S::IntoStream, N>;

    fn merge(self) -> Self::Stream {
        Merge::new(self.map(|i| i.into_stream()))
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
    fn merge_array_4() {
        block_on(async {
            let a = stream::once(1);
            let b = stream::once(2);
            let c = stream::once(3);
            let d = stream::once(4);
            let mut s = [a, b, c, d].merge();

            let mut counter = 0;
            while let Some(n) = s.next().await {
                counter += n;
            }
            assert_eq!(counter, 10);
        })
    }

    #[test]
    fn merge_array_2x2() {
        block_on(async {
            let a = stream::repeat(1).take(2);
            let b = stream::repeat(2).take(2);
            let mut s = [a, b].merge();

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
                        [receive1, receive2, receive3]
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
