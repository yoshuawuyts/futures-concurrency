use super::Race as RaceTrait;

use core::fmt::{self, Debug};
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

macro_rules! impl_race_tuple {
    ($StructName:ident $($F:ident)+) => {
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

        impl<T, $($F),*> RaceTrait for ($($F),*)
        where $(
            $F: IntoFuture<Output = T>,
        )* {
            type Output = T;
            type Future = $StructName<T, $($F::IntoFuture),*>;

            fn race(self) -> Self::Future {
                let ($($F),*): ($($F),*) = self;
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
                let this = self.project();
                assert!(!*this.done, "Futures must not be polled after completing");

                $( if let Poll::Ready(output) = Future::poll(this.$F, cx) {
                    *this.done = true;
                    return Poll::Ready(output);
                })*

                Poll::Pending
            }
        }
    };
}

impl_race_tuple! { Join2 A B }
impl_race_tuple! { Join3 A B C }
impl_race_tuple! { Join4 A B C D }
impl_race_tuple! { Join5 A B C D E }
impl_race_tuple! { Join6 A B C D E F }
impl_race_tuple! { Join7 A B C D E F G }
impl_race_tuple! { Join8 A B C D E F G H }
impl_race_tuple! { Join9 A B C D E F G H I }
impl_race_tuple! { Join10 A B C D E F G H I J }
impl_race_tuple! { Join11 A B C D E F G H I J K }
impl_race_tuple! { Join12 A B C D E F G H I J K L }

#[cfg(test)]
mod test {
    use super::*;
    use std::future;

    // NOTE: we should probably poll in random order.
    #[test]
    fn no_fairness() {
        futures_lite::future::block_on(async {
            let res = (future::ready("hello"), future::ready("world"))
                .race()
                .await;
            assert_eq!(res, "hello");
        });
    }

    #[test]
    fn thruple() {
        futures_lite::future::block_on(async {
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
