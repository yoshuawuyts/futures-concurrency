use super::Join as JoinTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

macro_rules! generate {
    ($(
        $(#[$doc:meta])*
        ($Join:ident, <$($Fut:ident),*>),
    )*) => ($(
        $(#[$doc])*
        #[pin_project]
        #[must_use = "futures do nothing unless you `.await` or poll them"]
        #[allow(non_snake_case)]
        pub struct $Join<$($Fut: Future),*> {
            $(#[pin] $Fut: MaybeDone<$Fut>,)*
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
            type Output = ($($Fut::Output),*);
            type Future = $Join<$($Fut),*>;

            fn join(self) -> Self::Future {
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
