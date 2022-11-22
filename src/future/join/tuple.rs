use super::Join as JoinTrait;
use crate::utils::{PollArray, WakerArray};

use core::fmt::{self, Debug};
use core::future::{Future, IntoFuture};
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

/// Generates the `poll` call for every `Future` inside `$futures`.
// This is implemented as a tt-muncher of the future name `$($F:ident)`
// and the future index `$($rest)`, taking advantage that we only support
// tuples up to  12 elements
//
// # References
// TT Muncher: https://veykril.github.io/tlborm/decl-macros/patterns/tt-muncher.html
macro_rules! poll {
    (@inner $iteration:ident, $this:ident, $futures:ident, $cx:ident, $fut_name:ident $($F:ident)* | $fut_idx:tt $($rest:tt)*) => {
        if $fut_idx == $iteration {
            if let Poll::Ready(value) = $futures.$fut_name.as_mut().poll(&mut $cx) {
                $this.outputs.$fut_idx.write(value);
                *$this.completed += 1;
                $this.state[$fut_idx].set_consumed();
            }
        }
        poll!(@inner $iteration, $this, $futures, $cx, $($F)* | $($rest)*);
    };

    // base condition, no more futures to poll
    (@inner $iteration:ident, $this:ident, $futures:ident, $cx:ident, | $($rest:tt)*) => {};

    ($iteration:ident, $this:ident, $futures:ident, $cx:ident, $LEN:ident, $($F:ident,)+) => {
        poll!(@inner $iteration, $this, $futures, $cx, $($F)+ | 0 1 2 3 4 5 6 7 8 9 10 11);
    };
}

macro_rules! impl_join_tuple {
    ($mod_name:ident $StructName:ident) => {
        /// Waits for two similarly-typed futures to complete.
        ///
        /// This `struct` is created by the [`join`] method on the [`Join`] trait. See
        /// its documentation for more.
        ///
        /// [`join`]: crate::future::Join::join
        /// [`Join`]: crate::future::Join
        #[must_use = "futures do nothing unless you `.await` or poll them"]
        #[allow(non_snake_case)]
        pub struct $StructName {}

        impl fmt::Debug for $StructName {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("Join").finish()
            }
        }

        impl Future for $StructName {
            type Output = ();

            fn poll(
                self: Pin<&mut Self>, _cx: &mut Context<'_>
            ) -> Poll<Self::Output> {
                Poll::Ready(())
            }
        }

        impl JoinTrait for () {
            type Output = ();
            type Future = $StructName;
            fn join(self) -> Self::Future {
                $StructName {}
            }
        }
    };
    ($mod_name:ident $StructName:ident $($F:ident)+) => {
        mod $mod_name {

            #[pin_project::pin_project]
            pub(super) struct Futures<$($F,)+> { $(#[pin] pub(super) $F: $F,)+ }

            #[repr(u8)]
            pub(super) enum Indexes { $($F,)+ }

            pub(super) const LEN: usize = [$(Indexes::$F,)+].len();
        }

        /// Waits for two similarly-typed futures to complete.
        ///
        /// This `struct` is created by the [`join`] method on the [`Join`] trait. See
        /// its documentation for more.
        ///
        /// [`join`]: crate::future::Join::join
        /// [`Join`]: crate::future::Join
        #[pin_project]
        #[must_use = "futures do nothing unless you `.await` or poll them"]
        #[allow(non_snake_case)]
        pub struct $StructName<$($F: Future),+> {
            #[pin] futures: $mod_name::Futures<$($F,)+>,
            outputs: ($(MaybeUninit<$F::Output>,)+),
            state: PollArray<{$mod_name::LEN}>,
            wakers: WakerArray<{$mod_name::LEN}>,
            completed: usize,
        }

        impl<$($F),+> Debug for $StructName<$($F),+>
        where $(
            $F: Future + Debug,
            $F::Output: Debug,
        )+ {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("Join")
                    $(.field(&self.futures.$F))+
                    .finish()
            }
        }

        #[allow(unused_mut)]
        #[allow(unused_parens)]
        #[allow(unused_variables)]
        impl<$($F: Future),+> Future for $StructName<$($F),+> {
            type Output = ($($F::Output,)+);

            fn poll(
                self: Pin<&mut Self>, cx: &mut Context<'_>
            ) -> Poll<Self::Output> {
                const LEN: usize = $mod_name::LEN;

                let mut this = self.project();
                let all_completed = !(*this.completed == LEN);
                assert!(all_completed, "Futures must not be polled after completing");

                let mut futures = this.futures.project();

                let mut readiness = this.wakers.readiness().lock().unwrap();
                readiness.set_waker(cx.waker());

                for index in 0..LEN {
                    if !readiness.any_ready() {
                        // nothing ready yet
                        return Poll::Pending;
                    }
                    if !readiness.clear_ready(index) || this.state[index].is_consumed() {
                        // future not ready yet or already polled to completion, skip
                        continue;
                    }

                    // unlock readiness so we don't deadlock when polling
                    drop(readiness);

                    // obtain the intermediate waker
                    let mut cx = Context::from_waker(this.wakers.get(index).unwrap());

                    // generate the needed code to poll `futures.{index}`
                    poll!(index, this, futures, cx, LEN, $($F,)+);

                    if *this.completed == LEN {
                        let out = {
                            let mut out = ($(MaybeUninit::<$F::Output>::uninit(),)+);
                            core::mem::swap(&mut out, this.outputs);
                            let ($($F,)+) = out;
                            unsafe { ($($F.assume_init(),)+) }
                        };

                        return Poll::Ready(out);
                    }
                    readiness = this.wakers.readiness().lock().unwrap();
                }

                Poll::Pending
            }
        }

        #[allow(unused_parens)]
        impl<$($F),+> JoinTrait for ($($F,)+)
        where $(
            $F: IntoFuture,
        )+ {
            type Output = ($($F::Output,)*);
            type Future = $StructName<$($F::IntoFuture),*>;

            fn join(self) -> Self::Future {
                let ($($F,)+): ($($F,)+) = self;
                $StructName {
                    futures: $mod_name::Futures {$($F: $F.into_future(),)+},
                    state: PollArray::new(),
                    outputs: ($(MaybeUninit::<$F::Output>::uninit(),)+),
                    wakers: WakerArray::new(),
                    completed: 0,
                }
            }
        }
    };
}

impl_join_tuple! { join0 Join0 }
impl_join_tuple! { join1 Join1 A }
impl_join_tuple! { join2 Join2 A B }
impl_join_tuple! { join3 Join3 A B C }
impl_join_tuple! { join4 Join4 A B C D }
impl_join_tuple! { join5 Join5 A B C D E }
impl_join_tuple! { join6 Join6 A B C D E F }
impl_join_tuple! { join7 Join7 A B C D E F G }
impl_join_tuple! { join8 Join8 A B C D E F G H }
impl_join_tuple! { join9 Join9 A B C D E F G H I }
impl_join_tuple! { join10 Join10 A B C D E F G H I J }
impl_join_tuple! { join11 Join11 A B C D E F G H I J K }
impl_join_tuple! { join12 Join12 A B C D E F G H I J K L }

#[cfg(test)]
mod test {
    use super::*;
    use std::future;

    #[test]
    #[allow(clippy::unit_cmp)]
    fn join_0() {
        futures_lite::future::block_on(async {
            assert_eq!(().join().await, ());
        });
    }

    #[test]
    fn join_1() {
        futures_lite::future::block_on(async {
            let a = future::ready("hello");
            assert_eq!((a,).join().await, ("hello",));
        });
    }

    #[test]
    fn join_2() {
        futures_lite::future::block_on(async {
            let a = future::ready("hello");
            let b = future::ready(12);
            assert_eq!((a, b).join().await, ("hello", 12));
        });
    }

    #[test]
    fn join_3() {
        futures_lite::future::block_on(async {
            let a = future::ready("hello");
            let b = future::ready("world");
            let c = future::ready(12);
            assert_eq!((a, b, c).join().await, ("hello", "world", 12));
        });
    }
}
