use super::Zip as ZipTrait;
use crate::stream::IntoStream;
use crate::utils::WakerArray;

use core::fmt;
use core::mem::MaybeUninit;
use futures_core::Stream;
use pin_project::{pin_project, pinned_drop};
use std::pin::Pin;
use std::task::{Context, Poll};

macro_rules! impl_zip_tuple {
    ($ignore:ident $StructName:ident) => {
        /// A stream that merges multiple streams into a single stream.
        ///
        /// This `struct` is created by the [`merge`] method on the [`Merge`] trait. See its
        /// documentation for more.
        ///
        /// [`merge`]: trait.Merge.html#method.merge
        /// [`Merge`]: trait.Merge.html
        pub struct $StructName {}

        impl fmt::Debug for $StructName {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("Merge").finish()
            }
        }

        impl Stream for $StructName {
            type Item = ();

            fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                Poll::Ready(Some(()))
            }
        }

        impl ZipTrait for () {
            type Item = ();
            type Stream = $StructName;

            fn zip(self) -> Self::Stream {
                $StructName { }
            }
        }
    };
    ($mod_name:ident $StructName:ident $($F:ident=$fut_idx:tt)+) => {
        mod $mod_name {
            #[pin_project::pin_project]
            pub(super) struct Streams<$($F,)+> { $(#[pin] pub(super) $F: $F),+ }

            pub(super) const LEN: usize = [$($fut_idx),+].len();
        }

        /// A stream that merges multiple streams into a single stream.
        ///
        /// This `struct` is created by the [`merge`] method on the [`Merge`] trait. See its
        /// documentation for more.
        ///
        /// [`merge`]: trait.Merge.html#method.merge
        /// [`Merge`]: trait.Merge.html
        #[pin_project(PinnedDrop)]
        pub struct $StructName<$($F),*>
        where $(
            $F: Stream,
        )* {
            #[pin] streams: $mod_name::Streams<$($F,)+>,
            items: ($(MaybeUninit<$F::Item>,)+),
            wakers: WakerArray<{$mod_name::LEN}>,
			filled: [bool; $mod_name::LEN],
            awake_list_buffer: [usize; $mod_name::LEN],
            pending: usize,
        }

        impl<$($F),*> fmt::Debug for $StructName<$($F),*>
        where $(
            $F: Stream + fmt::Debug,
        )* {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("Merge")
                    $( .field(&self.streams.$F) )* // Hides implementation detail of Streams struct
                    .finish()
            }
        }

        impl<$($F),*> Stream for $StructName<$($F),*>
        where $(
            $F: Stream,
        )* {
            type Item = ($($F::Item,)+);

            fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                let this = self.project();

				let num_awake = {
					let mut awakeness = this.wakers.awakeness();
					awakeness.set_parent_waker(cx.waker());
					let num_awake = if *this.pending == usize::MAX {
						*this.pending = $mod_name::LEN;
						*this.awake_list_buffer = core::array::from_fn(core::convert::identity);
						$mod_name::LEN
					}
					else {
						let awake_list = awakeness.awake_list();
						let num_awake = awake_list.len();
						this.awake_list_buffer[..num_awake].copy_from_slice(awake_list);
						num_awake
					};
					awakeness.clear();
					num_awake
				};

                let mut streams = this.streams.project();

                for &idx in this.awake_list_buffer.iter().take(num_awake) {
					let filled = &mut this.filled[idx];
					if *filled {
						continue;
					}
                    let mut cx = Context::from_waker(this.wakers.get(idx).unwrap());

                    match idx {
                        $(
                            $fut_idx => {
                                match unsafe { Pin::new_unchecked(&mut streams.$F) }.poll_next(&mut cx) {
									Poll::Ready(Some(value)) => {
										this.items.$fut_idx.write(value);
                                        *filled = true;
                                        *this.pending -= 1;
									}
                                    Poll::Ready(None) => {
                                        return Poll::Ready(None);
                                    }
                                    Poll::Pending => {}
								}
                            }
                        ),+
                        _ => unreachable!()
                    };
                }
                if *this.pending == 0 {
                    for filled in this.filled.iter_mut() {
                        debug_assert!(
                            *filled,
                            "The items array should have been filled"
                        );
                        *filled = false;
                    }
                    *this.pending = usize::MAX;
                    let mut out = ($(MaybeUninit::<$F::Item>::uninit(),)+);
                    core::mem::swap(&mut out, this.items);
                    Poll::Ready(Some(unsafe { ($(out.$fut_idx.assume_init(),)+) }))
                }
                else {
                    Poll::Pending
                }
            }
        }

        impl<$($F),*> ZipTrait for ($($F,)*)
        where $(
            $F: IntoStream,
        )* {
            type Item = ($($F::Item,)+);
            type Stream = $StructName<$($F::IntoStream),*>;

            fn zip(self) -> Self::Stream {
                let ($($F,)*): ($($F,)*) = self;
                $StructName {
                    streams: $mod_name::Streams { $($F: $F.into_stream()),+ },
                    items: ($(MaybeUninit::<$F::Item>::uninit(),)+),
                    wakers: WakerArray::new(),
                    filled: [false; $mod_name::LEN],
                    awake_list_buffer: [0; $mod_name::LEN],
                    pending: $mod_name::LEN,
                }
            }
        }
        #[pinned_drop]
        impl<$($F),*> PinnedDrop for $StructName<$($F),*>
        where $(
            $F: Stream,
        )* {
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

impl_zip_tuple! { zip0 Zip0 }
impl_zip_tuple! { zip1 Zip1 A=0 }
impl_zip_tuple! { zip2 Zip2 A=0 B=1 }
impl_zip_tuple! { zip3 Zip3 A=0 B=1 C=2 }
impl_zip_tuple! { zip4 Zip4 A=0 B=1 C=2 D=3 }
impl_zip_tuple! { zip5 Zip5 A=0 B=1 C=2 D=3 E=4 }
impl_zip_tuple! { zip6 Zip6 A=0 B=1 C=2 D=3 E=4 F=5 }
impl_zip_tuple! { zip7 Zip7 A=0 B=1 C=2 D=3 E=4 F=5 G=6 }
impl_zip_tuple! { zip8 Zip8 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 }
impl_zip_tuple! { zip9 Zip9 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 I=8 }
impl_zip_tuple! { zip10 Zip10 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 I=8 J=9 }
impl_zip_tuple! { zip11 Zip11 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 I=8 J=9 K=10 }
impl_zip_tuple! { zip12 Zip12 A=0 B=1 C=2 D=3 E=4 F=5 G=6 H=7 I=8 J=9 K=10 L=11 }
