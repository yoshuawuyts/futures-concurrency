use super::Join as JoinTrait;
use crate::utils::MaybeDone;

use core::fmt::{self, Debug};
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

macro_rules! impl_merge_tuple {
    ($StructName:ident $($F:ident)+) => {
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
            done: bool,
            $(#[pin] $F: MaybeDone<$F>,)*
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

        impl<$($F: Future),*> Future for $StructName<$($F),*> {
            type Output = ($($F::Output),*);

            fn poll(
                self: Pin<&mut Self>, cx: &mut Context<'_>
            ) -> Poll<Self::Output> {
                let mut all_done = true;
                let mut this = self.project();
                assert!(!*this.done, "Futures must not be polled after completing");

                $(all_done &= this.$F.as_mut().poll(cx).is_ready();)*

                if all_done {
                    *this.done = true;
                    Poll::Ready(($(this.$F.take().unwrap()),*))
                } else {
                    Poll::Pending
                }
            }
        }

        impl<$($F),*> JoinTrait for ($($F),*)
        where $(
            $F: IntoFuture,
        )* {
            type Output = ($($F::Output),*);
            type Future = $StructName<$($F::IntoFuture),*>;

            fn join(self) -> Self::Future {
                let ($($F),*): ($($F),*) = self;
                $StructName {
                    done: false,
                    $($F: MaybeDone::new($F.into_future())),*
                }
            }
        }
    };
}

impl_merge_tuple! { Join2 A B }
impl_merge_tuple! { Join3 A B C }
impl_merge_tuple! { Join4 A B C D }
impl_merge_tuple! { Join5 A B C D E }
impl_merge_tuple! { Join6 A B C D E F }
impl_merge_tuple! { Join7 A B C D E F G }
impl_merge_tuple! { Join8 A B C D E F G H }
impl_merge_tuple! { Join9 A B C D E F G H I }
impl_merge_tuple! { Join10 A B C D E F G H I J }
impl_merge_tuple! { Join11 A B C D E F G H I J K }
impl_merge_tuple! { Join12 A B C D E F G H I J K L }

#[cfg(test)]
mod test {
    use super::*;
    use std::future;

    #[test]
    fn join_2() {
        futures_lite::future::block_on(async {
            let res = (future::ready("hello"), future::ready(12)).join().await;
            assert_eq!(res, ("hello", 12));
        });
    }

    #[test]
    fn join_3() {
        futures_lite::future::block_on(async {
            let res = (
                future::ready("hello"),
                future::ready("world"),
                future::ready(12),
            )
                .join()
                .await;
            assert_eq!(res, ("hello", "world", 12));
        });
    }
}
