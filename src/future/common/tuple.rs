use crate::utils::WakerArray;

use core::fmt::{self, Debug};
use core::future::Future;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_core::TryFuture;

// Basically we're implementing try_join here.
// All the other combinators can be derived from try_join by wrapping subfutures
// to return Ok or Error appropriately, and mapping the final result.
//
// For example, race_ok wants to break upon the first Ok subfuture completion,
// so it would map subfuture Ok to Err, causing this underlying try_join to break.
// Then it would take the Err result and convert it back to Ok.
pub trait CombineTuple {
    type Combined;
    fn combine(self) -> Self::Combined;
}

// The final result conversion could be done by derived combinators,
// but providing the facilities to do it here helps reduce the number of poll layers needed.
pub trait MapResult<IntermediateResult> {
    type FinalResult;
    fn to_final_result(result: IntermediateResult) -> Self::FinalResult;
}

macro_rules! impl_common_tuple {
	($mod_name:ident $StructName:ident $SelectedFrom:ident $($F:ident=$idx:tt)+) => {
		mod $mod_name {
			#[pin_project::pin_project]
            pub(super) struct Futures<$($F,)+> { $(#[pin] pub(super) $F: $F,)+ }
            pub(super) const LEN: usize = [$($idx,)+].len();
		}
		#[pin_project::pin_project(PinnedDrop)]
        #[must_use = "futures do nothing unless you `.await` or poll them"]
        #[allow(non_snake_case)]
        pub struct $StructName<B, $($F: TryFuture),+> {
            pending: usize,
            items: ($(MaybeUninit<$F::Ok>,)+),
            wakers: WakerArray<{$mod_name::LEN}>,
            filled: [bool; $mod_name::LEN],
            awake_list_buffer: [usize; $mod_name::LEN],
            #[pin]
            futures: $mod_name::Futures<$($F,)+>,
            phantom: PhantomData<B>
        }

        impl<B, $($F),+> Debug for $StructName<B, $($F),+>
        where $(
            $F: TryFuture + Debug,
        )+ {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("Join")
                    $(.field(&self.futures.$F))+
                    .finish()
            }
        }

		impl<B, $($F),+> CombineTuple for (($($F,)+), B)
		where $(
            $F: TryFuture,
        )+ {
			type Combined = $StructName<B, $($F,)+>;
			fn combine(self) -> Self::Combined {
				$StructName {
                    filled: [false; $mod_name::LEN],
                    items: ($(MaybeUninit::<$F::Ok>::uninit(),)+),
                    wakers: WakerArray::new(),
                    pending: $mod_name::LEN,
                    awake_list_buffer: [0; $mod_name::LEN],
                    futures: $mod_name::Futures {$($F: self.0.$idx,)+},
                    phantom: PhantomData
                }
			}
		}

        #[allow(unused_mut)]
        #[allow(unused_parens)]
        #[allow(unused_variables)]
        impl<B: MapResult<Result<($($F::Ok,)+), select_types::$SelectedFrom<$($F::Error),+>>>, $($F: TryFuture),+> Future for $StructName<B, $($F),+> {
            type Output = B::FinalResult;

            fn poll(
                self: Pin<&mut Self>, cx: &mut Context<'_>
            ) -> Poll<Self::Output> {
                let mut this = self.project();
                assert!(
                    *this.pending > 0,
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
                            $idx => {
                                if let Poll::Ready(value) = futures.$F.as_mut().try_poll(&mut cx) {
									match value {
										Err(ret) => {
                                            let ret = Err(select_types::$SelectedFrom::$F(ret));
                                            let ret = B::to_final_result(ret);
											return Poll::Ready(ret);
										},
										Ok(store) => {
											this.items.$idx.write(store);
											true
										}
									}
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
                    debug_assert!(this.filled.iter().all(|&filled| filled), "Future should have filled items array");
                    this.filled.fill(false);
                    let out = {
                        let mut out = ($(MaybeUninit::<$F::Ok>::uninit(),)+);
                        core::mem::swap(&mut out, this.items);
                        let ($($F,)+) = out;
                        // SAFETY: we've checked with the state that all of our outputs have been
                        // filled, which means we're ready to take the data and assume it's initialized.
                        unsafe { ($($F.assume_init(),)+) }
                    };
                    Poll::Ready(B::to_final_result(Ok(out)))
                }
                else {
                    Poll::Pending
                }
            }
        }

        #[pin_project::pinned_drop]
        impl<B, $($F: TryFuture),+> PinnedDrop for $StructName<B, $($F),+> {
            fn drop(self: Pin<&mut Self>) {
                let this = self.project();
                $(
                    if this.filled[$idx] {
                        // SAFETY: we've just filtered down to *only* the initialized values.
                        // We can assume they're initialized, and this is where we drop them.
                        unsafe { this.items.$idx.assume_init_drop() };
                    }
                )+
            }
        }
	};
}

impl_common_tuple! { common1 CombinatorTuple1 SelectedFrom1 A0=0 }
impl_common_tuple! { common2 CombinatorTuple2 SelectedFrom2 A0=0 A1=1 }
impl_common_tuple! { common3 CombinatorTuple3 SelectedFrom3 A0=0 A1=1 A2=2 }
impl_common_tuple! { common4 CombinatorTuple4 SelectedFrom4 A0=0 A1=1 A2=2 A3=3 }
impl_common_tuple! { common5 CombinatorTuple5 SelectedFrom5 A0=0 A1=1 A2=2 A3=3 A4=4 }
impl_common_tuple! { common6 CombinatorTuple6 SelectedFrom6 A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 }
impl_common_tuple! { common7 CombinatorTuple7 SelectedFrom7 A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 A6=6 }
impl_common_tuple! { common8 CombinatorTuple8 SelectedFrom8 A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 A6=6 A7=7 }
impl_common_tuple! { common9 CombinatorTuple9 SelectedFrom9 A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 A6=6 A7=7 A8=8 }
impl_common_tuple! { common10 CombinatorTuple10 SelectedFrom10 A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 A6=6 A7=7 A8=8 A9=9 }
impl_common_tuple! { common11 CombinatorTuple11 SelectedFrom11 A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 A6=6 A7=7 A8=8 A9=9 A10=10 }
impl_common_tuple! { common12 CombinatorTuple12 SelectedFrom12 A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 A6=6 A7=7 A8=8 A9=9 A10=10 A11=11 }

/// Enum representing a single field in a tuple.
/// Implemented for tuples of size 1 through 12.
pub mod select_types {
    type Useless<T, const X: usize> = T;
    macro_rules! make_select_type {
        ($TypeName:ident, $($V:ident=$idx:literal)+) => {
            /// Enum representing a single field in a tuple.
            /// Variants are ordered from first field to last field.
            #[derive(Debug)]
            pub enum $TypeName<$($V),+> {
                $(
                    /// A tuple field.
                    $V($V)
                ),+
            }
            impl<T> $TypeName<$(Useless<T, $idx>),+> {
                /// Get the value if all fields have the same type.
                ///
                /// When the tuple is of form (T, T, T, T, ...),
                /// the value will be type T regardless of which field,
                /// so we can just get the T.
                pub fn any(self) -> T {
                    match self {
                        $(
                            $TypeName::$V(t) => t,
                        )+
                    }
                }
            }
        };
    }
    make_select_type! { SelectedFrom1, A0=0 }
    make_select_type! { SelectedFrom2, A0=0 A1=1 }
    make_select_type! { SelectedFrom3, A0=0 A1=1 A2=2 }
    make_select_type! { SelectedFrom4, A0=0 A1=1 A2=2 A3=3 }
    make_select_type! { SelectedFrom5, A0=0 A1=1 A2=2 A3=3 A4=4 }
    make_select_type! { SelectedFrom6, A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 }
    make_select_type! { SelectedFrom7, A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 A6=6 }
    make_select_type! { SelectedFrom8, A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 A6=6 A7=7 }
    make_select_type! { SelectedFrom9, A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 A6=6 A7=7 A8=8 }
    make_select_type! { SelectedFrom10, A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 A6=6 A7=7 A8=8 A9=9 }
    make_select_type! { SelectedFrom11, A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 A6=6 A7=7 A8=8 A9=9 A10=10 }
    make_select_type! { SelectedFrom12, A0=0 A1=1 A2=2 A3=3 A4=4 A5=5 A6=6 A7=7 A8=8 A9=9 A10=10 A11=11 }
}
