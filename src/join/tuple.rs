use super::Join as JoinTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

macro_rules! generate {
    ($(
        ($TyName:ident, <$($Fut:ident),*>),
    )*) => ($(
        #[pin_project]
        #[must_use = "futures do nothing unless you `.await` or poll them"]
        #[allow(non_snake_case)]
        pub struct $TyName<$($Fut: Future),*> {
            $(#[pin] $Fut: MaybeDone<$Fut>,)*
        }

        impl<$($Fut),*> fmt::Debug for $TyName<$($Fut),*>
        where
            $(
                $Fut: Future + fmt::Debug,
                $Fut::Output: fmt::Debug,
            )*
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($TyName))
                    $(.field(stringify!($Fut), &self.$Fut))*
                    .finish()
            }
        }

        impl<$($Fut),*> JoinTrait for ($($Fut),*)
        where
            $(
                $Fut: IntoFuture,
            )*
        {
            type Output = ($($Fut::Output),*);
            type Future = $TyName<$($Fut::IntoFuture),*>;

            fn join(self) -> Self::Future {
                let ($($Fut),*): ($($Fut),*) = self;
                $TyName {
                    $($Fut: MaybeDone::new($Fut.into_future())),*
                }
            }
        }

        impl<$($Fut: Future),*> Future for $TyName<$($Fut),*> {
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
    (Join2, <A, B>),
    (Join3, <A, B, C>),
    (Join4, <A, B, C, D>),
    (Join5, <A, B, C, D, E>),
    (Join6, <A, B, C, D, E, F>),
    (Join7, <A, B, C, D, E, F, G>),
    (Join8, <A, B, C, D, E, F, G, H>),
    (Join9, <A, B, C, D, E, F, G, H, I>),
    (Join10, <A, B, C, D, E, F, G, H, I, J>),
    (Join11, <A, B, C, D, E, F, G, H, I, J, K>),
    (Join12, <A, B, C, D, E, F, G, H, I, J, K, L>),
}
