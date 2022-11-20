use super::Join as JoinTrait;
use crate::utils::{PollArray, RandomGenerator, WakerArray};

use core::fmt::{self, Debug};
use core::future::{Future, IntoFuture};
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

macro_rules! poll_future {
    ($fut_idx:tt, $iteration:ident, $this:ident, $outputs:ident, $futures:ident . $fut_member:ident, $cx:ident) => {
        if $fut_idx == $iteration {
            if let Poll::Ready(value) =
                unsafe { Pin::new_unchecked(&mut $futures.$fut_member) }.poll(&mut $cx)
            {
                $this.outputs.$fut_member.write(value);
                *$this.completed += 1;
                $this.state[$fut_idx].set_consumed();
            }
        }
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
            use core::mem::MaybeUninit;
            use core::future::Future;

            #[pin_project::pin_project]
            pub(super) struct Futures<$($F,)+> { $(#[pin] pub(super) $F: $F,)+ }

            pub(super) struct Outputs<$($F: Future,)+> { $(pub(super) $F: MaybeUninit<$F::Output>,)+ }

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
            outputs: $mod_name::Outputs<$($F,)+>,
            rng: RandomGenerator,
            wakers: WakerArray<{$mod_name::LEN}>,
            state: PollArray<{$mod_name::LEN}>,
            completed: u8,
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
                let this = self.project();

                let mut readiness = this.wakers.readiness().lock().unwrap();
                readiness.set_waker(cx.waker());

                const LEN: u8 = $mod_name::LEN as u8;
                let r = this.rng.generate(LEN as u32) as u8;

                let mut futures = this.futures.project();

                for index in (0..LEN).map(|n| (r + n).wrapping_rem(LEN) as usize) {
                    if !readiness.any_ready() {
                        return Poll::Pending;
                    } else if !readiness.clear_ready(index) || this.state[index].is_consumed() {
                        continue;
                    }

                    drop(readiness);

                    let mut cx = Context::from_waker(this.wakers.get(index).unwrap());

                    $(
                        let fut_index = $mod_name::Indexes::$F as usize;
                        poll_future!(
                            fut_index,
                            index,
                            this,
                            outputs,
                            futures . $F,
                            cx
                        );
                    )+

                    if *this.completed == LEN {
                        let out = {
                            let mut output = $mod_name::Outputs { $($F: MaybeUninit::uninit(),)+ };
                            core::mem::swap(this.outputs, &mut output);
                            unsafe { ( $(output.$F.assume_init(),)+ ) }
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
                    futures: $mod_name::Futures { $($F: $F.into_future(),)+ },
                    rng: RandomGenerator::new(),
                    wakers: WakerArray::new(),
                    state: PollArray::new(),
                    outputs: $mod_name::Outputs { $($F: MaybeUninit::uninit(),)+ },
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
