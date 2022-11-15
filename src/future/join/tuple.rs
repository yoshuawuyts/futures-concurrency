use super::Join as JoinTrait;
use crate::utils::{self, PollState, PollStates};

use core::fmt::{self, Debug};
use core::future::{Future, IntoFuture};
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

macro_rules! maybe_poll {
    ($idx:tt, $len:ident, $this:ident, $fut:ident, $cx:ident) => {
        if $this.states[$idx].is_pending() {
            if let Poll::Ready(out) = $this.$fut.poll($cx) {
                $this.outputs.$idx = MaybeUninit::new(out);
                $this.states[$idx] = PollState::Done;
                *$this.len -= 1;
            }
        }
    };
}

macro_rules! poll_all_pending {
    (@inner 0, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(0, $len, $this, $fut, $cx);
        poll_all_pending!(@inner 1, $len, $this, $cx, ($($rest,)*));
    };
    (@inner 1, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(1, $len, $this, $fut, $cx);
        poll_all_pending!(@inner 2, $len, $this, $cx, ($($rest,)*));
    };
    (@inner 2, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(2, $len, $this, $fut, $cx);
        poll_all_pending!(@inner 3, $len, $this, $cx, ($($rest,)*));
    };
    (@inner 3, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(3, $len, $this, $fut, $cx);
        poll_all_pending!(@inner 4, $len, $this, $cx, ($($rest,)*));
    };
    (@inner 4, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(4, $len, $this, $fut, $cx);
        poll_all_pending!(@inner 5, $len, $this, $cx, ($($rest,)*));
    };
    (@inner 5, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(5, $len, $this, $fut, $cx);
        poll_all_pending!(@inner 6, $len, $this, $cx, ($($rest,)*));
    };
    (@inner 6, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(6, $len, $this, $fut, $cx);
        poll_all_pending!(@inner 7, $len, $this, $cx, ($($rest,)*));
    };
    (@inner 7, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(7, $len, $this, $fut, $cx);
        poll_all_pending!(@inner 8, $len, $this, $cx, ($($rest,)*));
    };
    (@inner 8, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(8, $len, $this, $fut, $cx);
        poll_all_pending!(@inner 9, $len, $this, $cx, ($($rest,)*));
    };
    (@inner 9, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(9, $len, $this, $fut, $cx);
        poll_all_pending!(@inner 10, $len, $this, $cx, ($($rest,)*));
    };
    (@inner 10, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(10, $len, $this, $fut, $cx);
        poll_all_pending!(@inner 11, $len, $this, $cx, ($($rest,)*));
    };
    (@inner 11, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(11, $len, $this, $fut, $cx);
        poll_all_pending!(@inner 12, $len, $this, $cx, ($($rest,)*));
    };
    (@inner 12, $len:ident, $this:ident, $cx:ident, ($fut:ident, $($rest:ident,)*)) => {
        maybe_poll!(12, $len, $this, $fut, $cx);
    };
    (@inner $ignore:literal, $len:ident, $this:ident, $cx:ident, ()) => { };
    ($len:ident, $this:ident, $cx:ident, $($F:ident,)*) => {
        poll_all_pending!(@inner 0, $len, $this, $cx, ($($F,)*));
    };
}

macro_rules! impl_join_tuple {
    ($StructName:ident $($F:ident)*) => {
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
        pub struct $StructName<$($F: Future),*> {
            len: u32,
            $(#[pin] $F: $F,)*
            outputs: ($(MaybeUninit<$F::Output>,)*),
            states: PollStates,
        }

        impl<$($F),*> Debug for $StructName<$($F),*>
        where $(
            $F: Future + Debug,
            $F::Output: Debug,
        )* {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("Join")
                    .field(&($(&self.$F,)*))
                    .field(&self.states)
                    .finish()
            }
        }

        #[allow(unused_mut)]
        #[allow(unused_parens)]
        #[allow(unused_variables)]
        impl<$($F: Future),*> Future for $StructName<$($F),*> {
            type Output = ($($F::Output,)*);

            fn poll(
                self: Pin<&mut Self>, cx: &mut Context<'_>
            ) -> Poll<Self::Output> {
                let mut this = self.project();

                poll_all_pending!(LEN, this, cx, $($F,)*);

                if *this.len <= 0 {
                    let out = unsafe {(this.outputs as *const _ as *const ($($F::Output,)*)).read()};
                    Poll::Ready(out)
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
            type Output = ($($F::Output,)*);
            type Future = $StructName<$($F::IntoFuture),*>;

            fn join(self) -> Self::Future {
                let ($($F,)*): ($($F,)*) = self;
                const LEN: u32 = utils::tuple_len!($($F,)*);
                $StructName {
                    len: LEN,
                    $($F: $F.into_future(),)*
                    outputs: ($(MaybeUninit::<$F::Output>::uninit(),)*),
                    states: PollStates::new(LEN as usize),
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
