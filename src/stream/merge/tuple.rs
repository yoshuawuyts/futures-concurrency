use super::Merge as MergeTrait;
use crate::stream::IntoStream;
use crate::utils::{ArrayDequeue, PollState, WakerArray};

use core::fmt;
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_core::Stream;

macro_rules! impl_merge_tuple {
    ($mod_name:ident $StructName:ident $($F:ident=$fut_idx:tt)+) => {
        mod $mod_name {
            #[pin_project::pin_project]
            pub(super) struct Streams<$($F,)+> { $(#[pin] pub(super) $F: $F),+ }

            pub(super) const LEN: usize = [$($fut_idx),+].len();
        }

        /// A stream that merges multiple streams into a single stream.
        ///
        /// This `struct` is created by the [`merge`] method on the [`Merge`] trait. See its
        /// documentation for more.
        ///
        /// [`merge`]: trait.Merge.html#method.merge
        /// [`Merge`]: trait.Merge.html
        #[pin_project::pin_project]
        pub struct $StructName<T, $($F),*>
        where $(
            $F: Stream<Item = T>,
        )* {
            #[pin] streams: $mod_name::Streams<$($F,)+>,
            wakers: WakerArray<{$mod_name::LEN}>,
            pending: usize,
            state: [PollState; $mod_name::LEN],
            awake_list: ArrayDequeue<usize, {$mod_name::LEN}>,
            #[cfg(debug_assertions)]
            done: bool
        }

        impl<T, $($F),*> fmt::Debug for $StructName<T, $($F),*>
        where $(
            $F: Stream<Item = T> + fmt::Debug,
            T: fmt::Debug,
        )* {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("Merge")
                    $( .field(&self.streams.$F) )* // Hides implementation detail of Streams struct
                    .finish()
            }
        }

        impl<T, $($F),*> Stream for $StructName<T, $($F),*>
        where $(
            $F: Stream<Item = T>,
        )* {
            type Item = T;

            fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                let this = self.project();

                #[cfg(debug_assertions)]
assert!(!*this.done, "Stream should not be polled after completing");

                {
                    let mut readiness = this.wakers.readiness();
                    readiness.set_parent_waker(cx.waker());
                    let awake_list = readiness.awake_list();
                    let states = &mut *this.state;
                    this.awake_list.extend(awake_list.iter().filter_map(|&idx| {
                        let state = &mut states[idx];
                        match state {
                            PollState::Pending => {
                                *state = PollState::Ready;
                                Some(idx)
                            },
                            _ => None
                        }
                    }));
                    readiness.clear();
                }

                let mut streams = this.streams.project();

                for idx in this.awake_list.drain() {
                    let state = &mut this.state[idx];
                    if let PollState::Consumed = *state {
                        continue;
                    }
                    let waker = this.wakers.get(idx).unwrap();
                    let mut cx = Context::from_waker(waker);

                    let poll_res = match idx {
                        $(
                            $fut_idx => {
                                streams.$F.as_mut().poll_next(&mut cx)
                            }
                        ),+
                        _ => unreachable!()
                    };
                    match poll_res {
                        Poll::Ready(Some(item)) => {
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
                }
                else {
                    Poll::Pending
                }
            }
        }

        impl<T, $($F),*> MergeTrait for ($($F,)*)
        where $(
            $F: IntoStream<Item = T>,
        )* {
            type Item = T;
            type Stream = $StructName<T, $($F::IntoStream),*>;

            fn merge(self) -> Self::Stream {
                let ($($F,)*): ($($F,)*) = self;
                $StructName {
                    streams: $mod_name::Streams { $($F: $F.into_stream()),+ },
                    wakers: WakerArray::new(),
                    pending: $mod_name::LEN,
                    state: [PollState::Ready; $mod_name::LEN],
                    awake_list: ArrayDequeue::new(core::array::from_fn(core::convert::identity), $mod_name::LEN),
                    #[cfg(debug_assertions)]
                    done: false
                }
            }
        }
    };
}
impl_merge_tuple! { merge1 Merge1 A=0 }
impl_merge_tuple! { merge2 Merge2 A=0 B=1 }
impl_merge_tuple! { merge3 Merge3 A=0 B=1 C=2 }
impl_merge_tuple! { merge4 Merge4 A=0 B=1 C=2 D=3 }
impl_merge_tuple! { merge5 Merge5 A=0 B=1 C=2 D=3 E=4 }
impl_merge_tuple! { merge6 Merge6 A=0 B=1 C=2 D=3 E=4 F=5 }
impl_merge_tuple! { merge7 Merge7 A=0 B=1 C=2 D=3 E=4 F=5 G=6 }
impl_merge_tuple! { merge8 Merge8 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 }
impl_merge_tuple! { merge9 Merge9 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 I=8 }
impl_merge_tuple! { merge10 Merge10 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 I=8 J=9 }
impl_merge_tuple! { merge11 Merge11 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 I=8 J=9 K=10 }
impl_merge_tuple! { merge12 Merge12 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 I=8 J=9 K=10 L=11 }

impl MergeTrait for () {
    type Item = core::convert::Infallible;
    type Stream = Merge0;
    fn merge(self) -> Self::Stream {
        Merge0
    }
}
#[derive(Debug)]
pub struct Merge0;
impl Stream for Merge0 {
    type Item = core::convert::Infallible;
    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::task::LocalSpawnExt;
    use futures_lite::future::block_on;
    use futures_lite::prelude::*;
    use futures_lite::stream;

    #[test]
    fn merge_tuple_0() {
        block_on(async {
            let mut s = ().merge();

            let mut called = false;
            while s.next().await.is_some() {
                called = true;
            }
            assert!(!called);
        })
    }

    #[test]
    fn merge_tuple_1() {
        block_on(async {
            let a = stream::once(1);
            let mut s = (a,).merge();

            let mut counter = 0;
            while let Some(n) = s.next().await {
                counter += n;
            }
            assert_eq!(counter, 1);
        })
    }

    #[test]
    fn merge_tuple_2() {
        block_on(async {
            let a = stream::once(1);
            let b = stream::once(2);
            let mut s = (a, b).merge();

            let mut counter = 0;
            while let Some(n) = s.next().await {
                counter += n;
            }
            assert_eq!(counter, 3);
        })
    }

    #[test]
    fn merge_tuple_3() {
        block_on(async {
            let a = stream::once(1);
            let b = stream::once(2);
            let c = stream::once(3);
            let mut s = (a, b, c).merge();

            let mut counter = 0;
            while let Some(n) = s.next().await {
                counter += n;
            }
            assert_eq!(counter, 6);
        })
    }

    #[test]
    fn merge_tuple_4() {
        block_on(async {
            let a = stream::once(1);
            let b = stream::once(2);
            let c = stream::once(3);
            let d = stream::once(4);
            let mut s = (a, b, c, d).merge();

            let mut counter = 0;
            while let Some(n) = s.next().await {
                counter += n;
            }
            assert_eq!(counter, 10);
        })
    }

    /// This test case uses channels so we'll have streams that return Pending from time to time.
    ///
    /// The purpose of this test is to make sure we have the waking logic working.
    #[test]
    fn merge_channels() {
        use std::cell::RefCell;
        use std::rc::Rc;

        use futures::executor::LocalPool;

        use crate::future::Join;
        use crate::utils::channel::local_channel;

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
                        (receive1, receive2, receive3)
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
