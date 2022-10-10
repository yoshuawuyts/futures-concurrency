use super::Join as JoinTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

macro_rules! generate {
    ($(
        (<$($Fut:ident),*>),
    )*) => ($( #[allow(non_snake_case)] const _: () = {
        #[pin_project]
        #[must_use = "futures do nothing unless you `.await` or poll them"]
        #[allow(non_snake_case)]
        pub struct Join<$($Fut: Future),*> {
            $(#[pin] $Fut: MaybeDone<$Fut>,)*
        }

        impl<$($Fut),*> fmt::Debug for Join<$($Fut),*>
        where
            $(
                $Fut: Future + fmt::Debug,
                $Fut::Output: fmt::Debug,
            )*
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!(Join))
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
            type Future = Join<$($Fut::IntoFuture),*>;

            fn join(self) -> Self::Future {
                let ($($Fut),*): ($($Fut),*) = self;
                Join {
                    $($Fut: MaybeDone::new($Fut.into_future())),*
                }
            }
        }

        impl<$($Fut: Future),*> Future for Join<$($Fut),*> {
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
    }; )*)
}

generate! {
    (<A, B>),
    (<A, B, C>),
    (<A, B, C, D>),
    (<A, B, C, D, E>),
    (<A, B, C, D, E, F>),
    (<A, B, C, D, E, F, G>),
    (<A, B, C, D, E, F, G, H>),
    (<A, B, C, D, E, F, G, H, I>),
    (<A, B, C, D, E, F, G, H, I, J>),
    (<A, B, C, D, E, F, G, H, I, J, K>),
    (<A, B, C, D, E, F, G, H, I, J, K, L>),
}
