use super::Join as JoinTrait;
use crate::utils::PollState;

use core::fmt::{self, Debug};
use core::future::{Future, IntoFuture};
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

macro_rules! impl_merge_tuple {
    ($mod_name:ident :: $StructName:ident $($F:ident)*) => {
        mod $mod_name {
            #[allow(unused_imports)] // Len == 0
            use super::*;

            pub(super) struct Outputs<$($F: Future),*> {
                $(pub(super) $F: MaybeUninit<$F::Output>),*
            }

            pub(super) struct States {
                $(pub(super) $F: PollState),*
            }
        }

        /// Waits for similarly-typed futures to complete.
        ///
        /// This `struct` is created by the [`join`] method on the [`Join`] trait. See
        /// its documentation for more.
        ///
        /// [`join`]: crate::future::Join::join
        /// [`Join`]: crate::future::Join
        #[pin_project]
        #[must_use = "futures do nothing unless you `.await` or poll them"]
        #[allow(non_snake_case)]
        pub struct $StructName<$($F: Future),*> {
            completed: usize,
            $(#[pin] $F: $F,)*
            outputs: $mod_name::Outputs<$($F),*>,
            states: $mod_name::States,
        }

        impl<$($F),*> Debug for $StructName<$($F),*>
        where $(
            $F: Future + Debug,
            $F::Output: Debug,
        )* {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("Join")
                    $(.field(&self.$F))*
                    .finish()
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
                const LEN: usize = {
                    // This will disappear after compilation, leaving only the usize constant but
                    // we need it to indicate a type for the slice content (for empty slices).
                    const ARRAY: &[&str] = &[$(stringify!($F)),*];
                    ARRAY.len()
                };

                let mut this = self.project();

                assert_ne!(*this.completed, LEN + 1, "Futures must not be polled after completing");

                $(
                    if this.states.$F.is_pending() {
                        if let Poll::Ready(value) = this.$F.poll(cx) {
                            this.outputs.$F.write(value);
                            this.states.$F = PollState::Done;
                            *this.completed += 1;
                        }
                    }
                )*

                if *this.completed < LEN {
                    Poll::Pending
                } else {
                    *this.completed += 1; // That way the 0-len case can be awaited once without problems
                    Poll::Ready(( $({
                        let mut out = MaybeUninit::uninit();
                        core::mem::swap(&mut this.outputs.$F, &mut out);
                        // Safety: we have reached `this.completed == LEN` so all futures have
                        // been successfully polled and the output written to `this.output`.
                        unsafe { out.assume_init() }
                    }),* ))
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
                $StructName {
                    completed: 0,
                    $($F: $F.into_future(),)*
                    outputs: $mod_name::Outputs { $($F: MaybeUninit::uninit()),* },
                    states: $mod_name::States { $($F: PollState::default()),* },
                }
            }
        }
    };
}

impl_merge_tuple! { join_0 :: Join0 }
impl_merge_tuple! { join_1 :: Join1 A }
impl_merge_tuple! { join_2 :: Join2 A B }
impl_merge_tuple! { join_3 :: Join3 A B C }
impl_merge_tuple! { join_4 :: Join4 A B C D }
impl_merge_tuple! { join_5 :: Join5 A B C D E }
impl_merge_tuple! { join_6 :: Join6 A B C D E F }
impl_merge_tuple! { join_7 :: Join7 A B C D E F G }
impl_merge_tuple! { join_8 :: Join8 A B C D E F G H }
impl_merge_tuple! { join_9 :: Join9 A B C D E F G H I }
impl_merge_tuple! { join_10 :: Join10 A B C D E F G H I J }
impl_merge_tuple! { join_11 :: Join11 A B C D E F G H I J K }
impl_merge_tuple! { join_12 :: Join12 A B C D E F G H I J K L }

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
