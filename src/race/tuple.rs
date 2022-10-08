use super::Race as RaceTrait;

use core::fmt;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

macro_rules! generate {
    ($(
        $(#[$doc:meta])*
        ($TyName:ident, <$($Fut:ident),*>),
    )*) => ($(
        $(#[$doc])*
        #[pin_project]
        #[must_use = "futures do nothing unless you `.await` or poll them"]
        #[allow(non_snake_case)]
        pub(crate) struct $TyName<T, $($Fut),*>
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

        #[async_trait::async_trait(?Send)]
        impl<T, $($Fut),*> RaceTrait for ($($Fut),*)
        where
            $($Fut: IntoFuture<Output = T>),*
        {
            type Output = T;

            async fn race(self) -> Self::Output {
                let ($($Fut),*): ($($Fut),*) = self;
                $TyName {
                    done: false,
                    $($Fut: $Fut.into_future()),*
                }.await
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
    /// Waits for two similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    (Race2, <A, B>),

    /// Waits for three similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    (Race3, <A, B, C>),

    /// Waits for four similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    (Race4, <A, B, C, D>),

    /// Waits for five similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    (Race5, <A, B, C, D, E>),

    /// Waits for six similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    (Race6, <A, B, C, D, E, F>),

    /// Waits for seven similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    (Race7, <A, B, C, D, E, F, G>),

    /// Waits for eight similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    (Race8, <A, B, C, D, E, F, G, H>),

    /// Waits for nine similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    (Race9, <A, B, C, D, E, F, G, H, I>),

    /// Waits for ten similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    (Race10, <A, B, C, D, E, F, G, H, I, J>),

    /// Waits for eleven similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    (Race11, <A, B, C, D, E, F, G, H, I, J, K>),

    /// Waits for twelve similarly-typed futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
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
