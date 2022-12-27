use super::Join as JoinTrait;
use crate::utils::WakerArray;

use core::fmt::{self, Debug};
use core::future::{Future, IntoFuture};
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::{pin_project, pinned_drop};

macro_rules! impl_join_tuple {
    ($mod_name:ident $StructName:ident) => {
        /// Waits for two similarly-typed futures to complete.
        ///
        /// This `struct` is created by the [`join`] method on the [`Join`] trait. See
        /// its documentation for more.
        ///
        /// [`join`]: crate::future::Join::join
        /// [`Join`]: crate::future::Join
        #[must_use = "futures do nothing unless you `.await` or poll them"]
        #[allow(non_snake_case)]
        pub struct $StructName {}

        impl fmt::Debug for $StructName {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("Join").finish()
            }
        }

        impl Future for $StructName {
            type Output = ();

            fn poll(
                self: Pin<&mut Self>, _cx: &mut Context<'_>
            ) -> Poll<Self::Output> {
                Poll::Ready(())
            }
        }

        impl JoinTrait for () {
            type Output = ();
            type Future = $StructName;
            fn join(self) -> Self::Future {
                $StructName {}
            }
        }
    };
    ($mod_name:ident $StructName:ident $($F:ident=$fut_idx:tt)+) => {
        mod $mod_name {

            #[pin_project::pin_project]
            pub(super) struct Futures<$($F,)+> { $(#[pin] pub(super) $F: $F,)+ }

            pub(super) const LEN: usize = [$($fut_idx,)+].len();
        }

        /// Waits for two similarly-typed futures to complete.
        ///
        /// This `struct` is created by the [`join`] method on the [`Join`] trait. See
        /// its documentation for more.
        ///
        /// [`join`]: crate::future::Join::join
        /// [`Join`]: crate::future::Join
        #[pin_project(PinnedDrop)]
        #[must_use = "futures do nothing unless you `.await` or poll them"]
        #[allow(non_snake_case)]
        pub struct $StructName<$($F: Future),+> {
            pending: usize,
            items: ($(MaybeUninit<$F::Output>,)+),
            wakers: WakerArray<{$mod_name::LEN}>,
            filled: [bool; $mod_name::LEN],
            awake_list_buffer: [usize; $mod_name::LEN],
            #[pin]
            futures: $mod_name::Futures<$($F,)+>,
        }

        impl<$($F),+> Debug for $StructName<$($F),+>
        where $(
            $F: Future + Debug,
            $F::Output: Debug,
        )+ {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("Join")
                    $(.field(&self.futures.$F))+
                    .finish()
            }
        }

        #[allow(unused_mut)]
        #[allow(unused_parens)]
        #[allow(unused_variables)]
        impl<$($F: Future),+> Future for $StructName<$($F),+> {
            type Output = ($($F::Output,)+);

            fn poll(
                self: Pin<&mut Self>, cx: &mut Context<'_>
            ) -> Poll<Self::Output> {
                const LEN: usize = $mod_name::LEN;

                let mut this = self.project();
                assert!(
                    LEN == 0 || *this.pending > 0,
                    "Futures must not be polled after completing"
                );

                let mut futures = this.futures.project();

                let num_awake = {
                    let mut awakeness = this.wakers.awakeness();
                    awakeness.set_parent_waker(cx.waker());
                    let awake_list = awakeness.awake_list();
                    let num_awake = awake_list.len();
                    this.awake_list_buffer[..num_awake].copy_from_slice(awake_list);
                    awakeness.clear();
                    num_awake
                };


                for &idx in this.awake_list_buffer.iter().take(num_awake) {
                    let filled = &mut this.filled[idx];
                    if *filled {
                        continue;
                    }
                    let mut cx = Context::from_waker(this.wakers.get(idx).unwrap());
                    let ready = match idx {
                        $(
                            $fut_idx => {
                                if let Poll::Ready(value) = futures.$F.as_mut().poll(&mut cx) {
                                    this.items.$fut_idx.write(value);
                                    true
                                }
                                else {
                                    false
                                }
                            },
                        )*
                        _ => unreachable!()
                    };
                    if ready {
                        *this.pending -= 1;
                        *filled = true;
                    }
                }

                if *this.pending == 0 {
                    let out = {
                        let mut out = ($(MaybeUninit::<$F::Output>::uninit(),)+);
                        core::mem::swap(&mut out, this.items);
                        let ($($F,)+) = out;
                        unsafe { ($($F.assume_init(),)+) }
                    };
                    Poll::Ready(out)
                }
                else {
                    Poll::Pending
                }
            }
        }

        #[allow(unused_parens)]
        impl<$($F),+> JoinTrait for ($($F,)+)
        where $(
            $F: IntoFuture,
        )+ {
            type Output = ($($F::Output,)*);
            type Future = $StructName<$($F::IntoFuture),*>;

            fn join(self) -> Self::Future {
                let ($($F,)+): ($($F,)+) = self;
                $StructName {
                    filled: [false; $mod_name::LEN],
                    items: ($(MaybeUninit::<$F::Output>::uninit(),)+),
                    wakers: WakerArray::new(),
                    pending: $mod_name::LEN,
                    awake_list_buffer: [0; $mod_name::LEN],
                    futures: $mod_name::Futures {$($F: $F.into_future(),)+},
                }
            }
        }

        #[pinned_drop]
        impl<$($F: Future),+> PinnedDrop for $StructName<$($F),+> {
            fn drop(self: Pin<&mut Self>) {
                let this = self.project();
                $(
                    if this.filled[$fut_idx] {
                        unsafe { this.items.$fut_idx.assume_init_drop() };
                    }
                )+
            }
        }
    };
}

impl_join_tuple! { join0 Join0 }
impl_join_tuple! { join1 Join1 A=0 }
impl_join_tuple! { join2 Join2 A=0 B=1 }
impl_join_tuple! { join3 Join3 A=0 B=1 C=2 }
impl_join_tuple! { join4 Join4 A=0 B=1 C=2 D=3 }
impl_join_tuple! { join5 Join5 A=0 B=1 C=2 D=3 E=4 }
impl_join_tuple! { join6 Join6 A=0 B=1 C=2 D=3 E=4 F=5 }
impl_join_tuple! { join7 Join7 A=0 B=1 C=2 D=3 E=4 F=5 G=6 }
impl_join_tuple! { join8 Join8 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 }
impl_join_tuple! { join9 Join9 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 I=8 }
impl_join_tuple! { join10 Join10 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 I=8 J=9 }
impl_join_tuple! { join11 Join11 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 I=8 J=9 K=10 }
impl_join_tuple! { join12 Join12 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 I=8 J=9 K=10 L=11 }

#[cfg(test)]
mod test {
    use super::*;
    use std::future;

    #[test]
    #[allow(clippy::unit_cmp)]
    fn join_0() {
        futures_lite::future::block_on(async {
            assert_eq!(().join().await, ());
        });
    }

    #[test]
    fn join_1() {
        futures_lite::future::block_on(async {
            let a = future::ready("hello");
            assert_eq!((a,).join().await, ("hello",));
        });
    }

    #[test]
    fn join_2() {
        futures_lite::future::block_on(async {
            let a = future::ready("hello");
            let b = future::ready(12);
            assert_eq!((a, b).join().await, ("hello", 12));
        });
    }

    #[test]
    fn join_3() {
        futures_lite::future::block_on(async {
            let a = future::ready("hello");
            let b = future::ready("world");
            let c = future::ready(12);
            assert_eq!((a, b, c).join().await, ("hello", "world", 12));
        });
    }
}
