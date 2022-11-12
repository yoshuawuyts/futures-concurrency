use super::Race as RaceTrait;
use crate::utils;

use core::fmt::{self, Debug};
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

/// Generate the `match` conditions inside the main `poll` body. This macro
/// chooses a random starting future on each `poll`, making it "fair".
//
/// The way this algorithm works is: we generate a random number between 0 and
/// the number of tuples we have. This number determines which stream we start
/// with. All other futures are mapped as `r + index`, and after we have the
/// first future, we'll sequentially iterate over all other futures. The
/// starting point of the future is random, but the iteration order of all other
/// futures is not.
///
// NOTE(yosh): this macro monstrocity is needed so we can increment each `else if` branch with
// + 1. When RFC 3086 becomes available to us, we can replace this with `${index($F)}` to get
// the current iteration.
//
// # References
// - https://twitter.com/maybewaffle/status/1588426440835727360
// - https://twitter.com/Veykril/status/1588231414998335490
// - https://rust-lang.github.io/rfcs/3086-macro-metavar-expr.html
macro_rules! gen_conditions {
    // Generate an `if`-block, and keep iterating.
    (@inner $LEN:expr, $i:expr, $r:expr, $this:expr, $cx:expr, $counter:expr, $F:ident, $($rest:ident,)*) => {
        if $i == ($r + $counter).wrapping_rem($LEN) {
            match unsafe { Pin::new_unchecked(&mut $this.$F) }.poll($cx) {
                Poll::Ready(output) => {
                    *$this.done = true;
                    return Poll::Ready(output);
                }
                _ => continue,
            }
        }
        gen_conditions!(@inner $LEN, $i, $r, $this, $cx, $counter + 1, $($rest,)*)
    };

    // End of recursion, nothing to do.
    (@inner $LEN:expr, $i:expr, $r:expr, $this:expr, $cx:expr, $counter:expr,) => {};

    // Base condition, setup the depth counter.
    ($LEN:expr, $i:expr, $r:expr, $this:expr, $cx:expr, $($F:ident,)*) => {
        gen_conditions!(@inner $LEN, $i, $r, $this, $cx, 0, $($F,)*)
    }
}

macro_rules! impl_race_tuple {
    ($StructName:ident $($F:ident)+) => {
        /// Wait for the first future to complete.
        ///
        /// This `struct` is created by the [`race`] method on the [`Race`] trait. See
        /// its documentation for more.
        ///
        /// [`race`]: crate::future::Race::race
        /// [`Race`]: crate::future::Race
        #[pin_project]
        #[must_use = "futures do nothing unless you `.await` or poll them"]
        #[allow(non_snake_case)]
        pub struct $StructName<T, $($F),*>
        where $(
            $F: Future<Output = T>,
        )* {
            done: bool,
            $(#[pin] $F: $F,)*
        }

        impl<T, $($F),*> Debug for $StructName<T, $($F),*>
        where $(
            $F: Future<Output = T> + Debug,
            T: Debug,
        )* {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("Race")
                    $(.field(&self.$F))*
                    .finish()
            }
        }

        impl<T, $($F),*> RaceTrait for ($($F,)*)
        where $(
            $F: IntoFuture<Output = T>,
        )* {
            type Output = T;
            type Future = $StructName<T, $($F::IntoFuture),*>;

            fn race(self) -> Self::Future {
                let ($($F,)*): ($($F,)*) = self;
                $StructName {
                    done: false,
                    $($F: $F.into_future()),*
                }
            }
        }

        impl<T, $($F: Future),*> Future for $StructName<T, $($F),*>
        where
            $($F: Future<Output = T>),*
        {
            type Output = T;

            fn poll(
                self: Pin<&mut Self>, cx: &mut Context<'_>
            ) -> Poll<Self::Output> {
                let mut this = self.project();
                assert!(!*this.done, "Futures must not be polled after completing");

                const LEN: u32 = utils::tuple_len!($($F,)*);
                const PERMUTATIONS: u32 = utils::permutations(LEN);
                let r = utils::random(PERMUTATIONS);

                for i in 0..LEN {
                    gen_conditions!(LEN, i, r, this, cx, $($F,)*);
                }

                Poll::Pending
            }
        }
    };
}

impl_race_tuple! { Race1 A }
impl_race_tuple! { Race2 A B }
impl_race_tuple! { Race3 A B C }
impl_race_tuple! { Race4 A B C D }
impl_race_tuple! { Race5 A B C D E }
impl_race_tuple! { Race6 A B C D E F }
impl_race_tuple! { Race7 A B C D E F G }
impl_race_tuple! { Race8 A B C D E F G H }
impl_race_tuple! { Race9 A B C D E F G H I }
impl_race_tuple! { Race10 A B C D E F G H I J }
impl_race_tuple! { Race11 A B C D E F G H I J K }
impl_race_tuple! { Race12 A B C D E F G H I J K L }

#[cfg(test)]
mod test {
    use super::*;
    use std::future;

    #[test]
    fn race_1() {
        futures_lite::future::block_on(async {
            let a = future::ready("world");
            assert_eq!((a,).race().await, "world");
        });
    }

    #[test]
    fn race_2() {
        futures_lite::future::block_on(async {
            let a = future::pending();
            let b = future::ready("world");
            assert_eq!((a, b).race().await, "world");
        });
    }

    #[test]
    fn race_3() {
        futures_lite::future::block_on(async {
            let a = future::pending();
            let b = future::ready("hello");
            let c = future::ready("world");
            let result = (a, b, c).race().await;
            assert!(matches!(result, "hello" | "world"));
        });
    }
}
