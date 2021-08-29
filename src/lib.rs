//! Concurrency extensions for `Future`.
//!
//! # Examples
//!
//! ```
//! use futures_lite::future::block_on;
//! use std::future::ready;
//! use futures_concurrency::prelude::*;
//!
//! fn main() {
//!     block_on(async {
//!         // Await multiple similarly-typed futures
//!         let fut = [ready(1u8), ready(2u8), ready(3u8)].join();
//!         let res = fut.await;
//!         println!("{} {} {}", res[0], res[1], res[2]);
//!
//!         // Await multiple differently-typed futures
//!         let fut = (ready(1u8), ready("hello"), ready(3u8)).join();
//!         let (a, b, c) = fut.await;
//!         println!("{} {} {}", a, b, c);
//!     })
//! }
//! ```
//!
//! # Progress
//!
//! The following traits have been implemented.
//!
//! - [x] `Join`
//! - [ ] `TryJoin`
//! - [ ] `Race`
//! - [ ] `TryRace`
//!
//! # Base Futures Concurrency
//!
//! Often it's desireable to await multiple futures as if it was a single
//! future. The `join` family of operations converts multiple futures into a
//! single future that returns all of their outputs. The `race` family of
//! operations converts multiple future into a single future that returns the
//! first output.
//!
//! For operating on futures the following functions can be used:
//!
//! | Name     | Return signature | When does it return?     |
//! | ---      | ---              | ---                      |
//! | `Join`   | `(T1, T2)`       | Wait for all to complete
//! | `Race`   | `T`              | Return on first value
//!
//! ## Fallible Futures Concurrency
//!
//! For operating on futures that return `Result` additional `try_` variants of
//! the functions mentioned before can be used. These functions are aware of `Result`,
//! and will behave slightly differently from their base variants.
//!
//! In the case of `try_join`, if any of the futures returns `Err` all
//! futures are dropped and an error is returned. This is referred to as
//! "short-circuiting".
//!
//! In the case of `try_race`, instead of returning the first future that
//! completes it returns the first future that _successfully_ completes. This
//! means `try_race` will keep going until any one of the futures returns
//! `Ok`, or _all_ futures have returned `Err`.
//!
//! However sometimes it can be useful to use the base variants of the functions
//! even on futures that return `Result`. Here is an overview of operations that
//! work on `Result`, and their respective semantics:
//!
//! | Name        | Return signature               | When does it return? |
//! | ---         | ---                            | ---                  |
//! | `Join`      | `(Result<T, E>, Result<T, E>)` | Wait for all to complete
//! | `TryJoin`   | `Result<(T1, T2), E>`          | Return on first `Err`, wait for all to complete
//! | `Race`      | `Result<T, E>`                 | Return on first value
//! | `Try_race`  | `Result<T, E>`                 | Return on first `Ok`, reject on last Err

#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(missing_docs, unreachable_pub)]
#![allow(non_snake_case)]

mod maybe_done;

use core::future::Future;
use std::pin::Pin;

pub(crate) use maybe_done::MaybeDone;

/// The futures concurrency prelude.
pub mod prelude {
    pub use super::Join;
}

/// Wait for multiple futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the futures
/// once both complete.
pub trait Join {
    /// The resulting joined future.
    type Output: Future;

    /// Waits for multiple futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the futures once both complete.
    ///
    /// This function returns a new future which polls both futures concurrently.
    fn join(self) -> Self::Output;
}

/// Implementations for the Array type.
pub mod array {
    use crate::iter_pin_mut;

    use super::{Join as JoinTrait, MaybeDone};

    use core::fmt;
    use core::future::Future;
    use core::mem;
    use core::pin::Pin;
    use core::task::{Context, Poll};
    use std::boxed::Box;
    use std::vec::Vec;

    impl<T, const N: usize> JoinTrait for [T; N]
    where
        T: Future,
    {
        type Output = Join<T>;

        fn join(self) -> Self::Output {
            Join {
                elems: Box::pin(self.map(MaybeDone::new)),
            }
        }
    }

    /// Waits for two similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Join<F>
    where
        F: Future,
    {
        elems: Pin<Box<[MaybeDone<F>]>>,
    }

    impl<F> fmt::Debug for Join<F>
    where
        F: Future + fmt::Debug,
        F::Output: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("JoinAll")
                .field("elems", &self.elems)
                .finish()
        }
    }

    impl<F> Future for Join<F>
    where
        F: Future,
    {
        type Output = Vec<F::Output>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut all_done = true;

            for elem in iter_pin_mut(self.elems.as_mut()) {
                if elem.poll(cx).is_pending() {
                    all_done = false;
                }
            }

            if all_done {
                let mut elems = mem::replace(&mut self.elems, Box::pin([]));
                let result = iter_pin_mut(elems.as_mut())
                    .map(|e| e.take().unwrap())
                    .collect();
                Poll::Ready(result)
            } else {
                Poll::Pending
            }
        }
    }
}

/// Implementations for the Vec type.
pub mod vec {
    use crate::iter_pin_mut;

    use super::{Join as JoinTrait, MaybeDone};

    use core::fmt;
    use core::future::Future;
    use core::mem;
    use core::pin::Pin;
    use core::task::{Context, Poll};
    use std::boxed::Box;
    use std::vec::Vec;

    impl<T> JoinTrait for Vec<T>
    where
        T: Future,
    {
        type Output = Join<T>;

        fn join(self) -> Self::Output {
            let elems: Box<[_]> = self.into_iter().map(MaybeDone::new).collect();
            Join {
                elems: elems.into(),
            }
        }
    }

    /// Waits for two similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Join<F>
    where
        F: Future,
    {
        elems: Pin<Box<[MaybeDone<F>]>>,
    }

    impl<F> fmt::Debug for Join<F>
    where
        F: Future + fmt::Debug,
        F::Output: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("JoinAll")
                .field("elems", &self.elems)
                .finish()
        }
    }

    impl<F> Future for Join<F>
    where
        F: Future,
    {
        type Output = Vec<F::Output>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut all_done = true;

            for elem in iter_pin_mut(self.elems.as_mut()) {
                if elem.poll(cx).is_pending() {
                    all_done = false;
                }
            }

            if all_done {
                let mut elems = mem::replace(&mut self.elems, Box::pin([]));
                let result = iter_pin_mut(elems.as_mut())
                    .map(|e| e.take().unwrap())
                    .collect();
                Poll::Ready(result)
            } else {
                Poll::Pending
            }
        }
    }
}

/// Implementations for the tuple type.
pub mod tuple {
    use super::{Join as JoinTrait, MaybeDone};

    use core::fmt;
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    use pin_project_lite::pin_project;

    macro_rules! generate {
        ($(
            $(#[$doc:meta])*
            ($Join:ident, <$($Fut:ident),*>),
        )*) => ($(
            pin_project! {
                $(#[$doc])*
                #[must_use = "futures do nothing unless you `.await` or poll them"]
                #[allow(non_snake_case)]
                pub struct $Join<$($Fut: Future),*> {
                    $(#[pin] $Fut: MaybeDone<$Fut>,)*
                }
            }

            impl<$($Fut),*> fmt::Debug for $Join<$($Fut),*>
            where
                $(
                    $Fut: Future + fmt::Debug,
                    $Fut::Output: fmt::Debug,
                )*
            {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.debug_struct(stringify!($Join))
                        $(.field(stringify!($Fut), &self.$Fut))*
                        .finish()
                }
            }

            impl<$($Fut: Future),*> $Join<$($Fut),*> {
                fn new(($($Fut),*): ($($Fut),*)) -> Self {
                    Self {
                        $($Fut: MaybeDone::new($Fut)),*
                    }
                }
            }

            impl<$($Fut),*> JoinTrait for ($($Fut),*)
            where
                $(
                    $Fut: Future,
                )*
            {
                type Output = $Join<$($Fut),*>;

                fn join(self) -> Self::Output {
                    $Join::new(self)
                }
            }

            impl<$($Fut: Future),*> Future for $Join<$($Fut),*> {
                type Output = ($($Fut::Output),*);

                fn poll(
                    self: Pin<&mut Self>, cx: &mut Context<'_>
                ) -> Poll<Self::Output> {
                    let mut all_done = true;
                    let mut futures = self.project();
                    $(
                        all_done &= futures.$Fut.as_mut().poll(cx).is_ready();
                    )*

                    if all_done {
                        Poll::Ready(($(futures.$Fut.take().unwrap()), *))
                    } else {
                        Poll::Pending
                    }
                }
            }
        )*)
    }

    generate! {
        /// Waits for two similarly-typed futures to complete.
        ///
        /// Awaits multiple futures simultaneously, returning the output of the
        /// futures once both complete.
        (Join2, <A, B>),

        /// Waits for three similarly-typed futures to complete.
        ///
        /// Awaits multiple futures simultaneously, returning the output of the
        /// futures once both complete.
        (Join3, <A, B, C>),

        /// Waits for four similarly-typed futures to complete.
        ///
        /// Awaits multiple futures simultaneously, returning the output of the
        /// futures once both complete.
        (Join4, <A, B, C, D>),

        /// Waits for five similarly-typed futures to complete.
        ///
        /// Awaits multiple futures simultaneously, returning the output of the
        /// futures once both complete.
        (Join5, <A, B, C, D, E>),

        /// Waits for six similarly-typed futures to complete.
        ///
        /// Awaits multiple futures simultaneously, returning the output of the
        /// futures once both complete.
        (Join6, <A, B, C, D, E, F>),

        /// Waits for seven similarly-typed futures to complete.
        ///
        /// Awaits multiple futures simultaneously, returning the output of the
        /// futures once both complete.
        (Join7, <A, B, C, D, E, F, G>),

        /// Waits for eight similarly-typed futures to complete.
        ///
        /// Awaits multiple futures simultaneously, returning the output of the
        /// futures once both complete.
        (Join8, <A, B, C, D, E, F, G, H>),

        /// Waits for nine similarly-typed futures to complete.
        ///
        /// Awaits multiple futures simultaneously, returning the output of the
        /// futures once both complete.
        (Join9, <A, B, C, D, E, F, G, H, I>),

        /// Waits for ten similarly-typed futures to complete.
        ///
        /// Awaits multiple futures simultaneously, returning the output of the
        /// futures once both complete.
        (Join10, <A, B, C, D, E, F, G, H, I, J>),

        /// Waits for eleven similarly-typed futures to complete.
        ///
        /// Awaits multiple futures simultaneously, returning the output of the
        /// futures once both complete.
        (Join11, <A, B, C, D, E, F, G, H, I, J, K>),

        /// Waits for twelve similarly-typed futures to complete.
        ///
        /// Awaits multiple futures simultaneously, returning the output of the
        /// futures once both complete.
        (Join12, <A, B, C, D, E, F, G, H, I, J, K, L>),
    }
}

pub(crate) fn iter_pin_mut<T>(slice: Pin<&mut [T]>) -> impl Iterator<Item = Pin<&mut T>> {
    // Safety: `std` _could_ make this unsound if it were to decide Pin's
    // invariants aren't required to transmit through slices. Otherwise this has
    // the same safety as a normal field pin projection.
    unsafe { slice.get_unchecked_mut() }
        .iter_mut()
        .map(|t| unsafe { Pin::new_unchecked(t) })
}
