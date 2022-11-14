use super::Join as JoinTrait;
use crate::utils::PollState;

use core::fmt::{self, Debug};
use core::future::{Future, IntoFuture};
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::{pin_project, pinned_drop};

use paste::paste;

macro_rules! impl_join_tuple {
    ($StructName:ident $($F:ident)*) => {
        paste!{
            /// Waits for two similarly-typed futures to complete.
            ///
            /// This `struct` is created by the [`join`] method on the [`Join`] trait. See
            /// its documentation for more.
            ///
            /// [`join`]: crate::future::Join::join
            /// [`Join`]: crate::future::Join
            #[pin_project(PinnedDrop)]
            #[must_use = "futures do nothing unless you `.await` or poll them"]
            #[allow(non_snake_case)]
            pub struct $StructName<$($F: Future),*> {
                done: bool,
                $(
                    #[pin] $F: $F,
                    [<$F _out>]: MaybeUninit<$F::Output>,
                    [<$F _state>]: PollState,
                )*
            }
        }

        impl<$($F),*> Debug for $StructName<$($F),*>
        where $(
            $F: Future + Debug,
            $F::Output: Debug,
        )* {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                paste!{
                    f.debug_tuple("Join")
                        $(.field(&self.$F).field(&self.[<$F _state >]))*
                        .finish()
                }
            }
        }

        #[allow(unused_mut)]
        #[allow(unused_parens)]
        #[allow(unused_variables)]
        impl<$($F: Future),*> Future for $StructName<$($F),*> {
            type Output = ($($F::Output),*);

            fn poll(
                self: Pin<&mut Self>, cx: &mut Context<'_>
            ) -> Poll<Self::Output> {
                let mut all_done = true;
                let mut this = self.project();
                assert!(!*this.done, "Futures must not be polled after completing");

                // Poll futures
                paste! {
                    $(
                        if this.[<$F _state>].is_pending() {
                            if let Poll::Ready(out) = this.$F.poll(cx) {
                                *this.[<$F _out>] = MaybeUninit::new(out);
                                *this.[<$F _state>] = PollState::Done;
                            }
                        }
                        all_done &= this.[<$F _state>].is_done();
                    )*
                }

                if all_done {
                    *this.done = true;
                    paste! {
                        Poll::Ready(($( unsafe { this.[<$F _out>].assume_init_read() }),*))
                    }
                } else {
                    Poll::Pending
                }
            }
        }

        #[allow(unused_parens)]
        impl<$($F),*> JoinTrait for ($($F,)*)
        where $(
            $F: IntoFuture,
        )* {
            type Output = ($($F::Output),*);
            type Future = $StructName<$($F::IntoFuture),*>;

            fn join(self) -> Self::Future {
                let ($($F,)*): ($($F,)*) = self;
                paste! {
                    $StructName {
                        done: false,
                        $(
                            $F: $F.into_future(),
                            [<$F _out>]: MaybeUninit::uninit(),
                            [<$F _state>]: PollState::default(),
                        )*
                    }
                }
            }
        }

        #[pinned_drop]
        impl<$($F,)*> PinnedDrop for $StructName<$($F,)*>
        where $(
            $F: Future,
        )* {
            fn drop(self: Pin<&mut Self>) {
                let _this = self.project();

                paste! {
                    $(
                        if _this.[<$F _state>].is_done() {
                            // SAFETY: if the future is marked as done, we can safelly drop its out
                            unsafe { _this.[<$F _out>].assume_init_drop() };
                        }
                    )*
                }
            }
        }
    };
}

impl_join_tuple! { Join0 }
impl_join_tuple! { Join1 A }
impl_join_tuple! { Join2 A B }
impl_join_tuple! { Join3 A B C }
impl_join_tuple! { Join4 A B C D }
impl_join_tuple! { Join5 A B C D E }
impl_join_tuple! { Join6 A B C D E F }
impl_join_tuple! { Join7 A B C D E F G }
impl_join_tuple! { Join8 A B C D E F G H }
impl_join_tuple! { Join9 A B C D E F G H I }
impl_join_tuple! { Join10 A B C D E F G H I J }
impl_join_tuple! { Join11 A B C D E F G H I J K }
impl_join_tuple! { Join12 A B C D E F G H I J K L }

#[cfg(test)]
mod test {
    use super::*;
    use std::future;

    #[test]
    fn join_0() {
        futures_lite::future::block_on(async {
            assert_eq!(().join().await, ());
        });
    }

    #[test]
    fn join_1() {
        futures_lite::future::block_on(async {
            let a = future::ready("hello");
            assert_eq!((a,).join().await, ("hello"));
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
