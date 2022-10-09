use super::Race as RaceTrait;

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
        pub struct $TyName<T, $($Fut),*>
        where
            $($Fut: Future<Output = T>),*
        {
            done: bool,
            $(#[pin] $Fut: $Fut,)*
        }

        impl<T, $($Fut),*> fmt::Debug for $TyName<T, $($Fut),*>
        where
            $(
                $Fut: Future<Output = T> + fmt::Debug,
                T: fmt::Debug,
            )*
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($TyName))
                    $(.field(stringify!($Fut), &self.$Fut))*
                    .finish()
            }
        }

        impl<T, $($Fut),*> RaceTrait for ($($Fut),*)
        where
            $($Fut: IntoFuture<Output = T>),*
        {
            type Output = T;
            type Future = $TyName<T, $($Fut::IntoFuture),*>;

            fn race(self) -> Self::Future {
                let ($($Fut),*): ($($Fut),*) = self;
                $TyName {
                    done: false,
                    $($Fut: $Fut.into_future()),*
                }
            }
        }

        impl<T, $($Fut: Future),*> Future for $TyName<T, $($Fut),*>
        where
            $($Fut: Future<Output = T>),*
        {
            type Output = T;

            fn poll(
                self: Pin<&mut Self>, cx: &mut Context<'_>
            ) -> Poll<Self::Output> {
                let this = self.project();

                assert!(
                    !*this.done,
                    "Futures must not be polled after being completed"
                );

                $(
                    if let Poll::Ready(output) = Future::poll(this.$Fut, cx) {
                        *this.done = true;
                        return Poll::Ready(output);
                    }
                )*
                Poll::Pending
            }
        }
    )*)
}

generate! {
    (Race2, <A, B>),
    (Race3, <A, B, C>),
    (Race4, <A, B, C, D>),
    (Race5, <A, B, C, D, E>),
    (Race6, <A, B, C, D, E, F>),
    (Race7, <A, B, C, D, E, F, G>),
    (Race8, <A, B, C, D, E, F, G, H>),
    (Race9, <A, B, C, D, E, F, G, H, I>),
    (Race10, <A, B, C, D, E, F, G, H, I, J>),
    (Race11, <A, B, C, D, E, F, G, H, I, J, K>),
    (Race12, <A, B, C, D, E, F, G, H, I, J, K, L>),
}

#[cfg(test)]
mod test {
    use super::*;
    use std::future;

    // NOTE: we should probably poll in random order.
    #[test]
    fn no_fairness() {
        async_io::block_on(async {
            let res = (future::ready("hello"), future::ready("world"))
                .race()
                .await;
            assert_eq!(res, "hello");
        });
    }

    #[test]
    fn thruple() {
        async_io::block_on(async {
            let res = (
                future::pending(),
                future::ready("world"),
                future::ready("hello"),
            )
                .race()
                .await;
            assert_eq!(res, "world");
        });
    }
}
