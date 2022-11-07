use super::Join as JoinTrait;
use crate::utils::MaybeDone;

use core::fmt::{self, Debug};
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::pin_project;

macro_rules! impl_merge_tuple {
    ($StructName:ident $($F:ident)+) => {
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
