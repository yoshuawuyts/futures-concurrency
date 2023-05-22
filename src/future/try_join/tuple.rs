use super::TryJoin;
use crate::utils::{self, PollArray};

use core::fmt;
use core::future::{Future, IntoFuture};
use core::mem::{self, MaybeUninit};
use core::pin::Pin;
use core::task::{Context, Poll};

use pin_project::{pin_project, pinned_drop};

macro_rules! impl_try_join_tuple {
    ($mod_name:ident $StructName:ident $(($F:ident $R:ident))+) => {
        mod $mod_name {
            pub(super) struct Output<$($R,)+>
            {
                $(pub(super) $R: core::mem::MaybeUninit<$R>,)+
            }

            impl<$($R,)+> Default for Output<$($R,)+>
            {
                fn default() -> Self {
                    Self {
                        $($R: core::mem::MaybeUninit::uninit(),)+
                    }
                }
            }

            #[repr(usize)]
            enum Indexes {
                $($F,)+
            }

            $(
                pub(super) const $F: usize = Indexes::$F as usize;
            )+

            pub(super) const LEN: usize = [$(Indexes::$F,)+].len();
        }

        /// A future which waits for all futures to complete successfully, or abort early on error.
        ///
        /// This `struct` is created by the [`try_join`] method on the [`TryJoin`] trait. See
        /// its documentation for more.
        ///
        /// [`try_join`]: crate::future::TryJoin::try_join
        /// [`TryJoin`]: crate::future::TryJoin
        #[must_use = "futures do nothing unless you `.await` or poll them"]
        #[allow(non_snake_case)]
        #[pin_project(PinnedDrop)]
        pub struct $StructName<$($F, $R,)+>
        {
            done: bool,
            completed: usize,
            indexer: utils::Indexer,
            output: $mod_name::Output<$($R,)+>,
            output_states: PollArray<{ $mod_name::LEN }>,
            $( #[pin] $F: $F, )+
        }

        impl<$($F, $R,)+> fmt::Debug for $StructName<$($F, $R,)+>
        where
            $($F: fmt::Debug,)+
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("TryJoin")
                    $(.field(&self.$F))+
                    .finish()
            }
        }

        impl<ERR, $($F, $R,)+> TryJoin for ($($F,)+)
        where
            $( $F: IntoFuture<Output = Result<$R, ERR>>, )+
        {
            type Output = ($($R,)+);
            type Error = ERR;
            type Future = $StructName<$($F::IntoFuture, $R,)+>;

            fn try_join(self) -> Self::Future {
                let ($($F,)+): ($($F,)+) = self;
                $StructName {
                    completed: 0,
                    done: false,
                    indexer: utils::Indexer::new($mod_name::LEN),
                    output: Default::default(),
                    output_states: PollArray::new(),
                    $($F: $F.into_future()),+
                }
            }
        }

        impl<ERR, $($F: Future, $R,)+> Future for $StructName<$($F, $R,)+>
        where
            $( $F: Future<Output = Result<$R, ERR>>, )+
        {
            type Output = Result<($($R,)+), ERR>;

            fn poll(
                self: Pin<&mut Self>, cx: &mut Context<'_>
            ) -> Poll<Self::Output> {
                let mut this = self.project();

                assert!(!*this.done, "Futures must not be polled after completing");

                for i in this.indexer.iter() {
                    if this.output_states[i].is_ready() {
                        continue;
                    }
                    utils::gen_conditions!(i, this, cx, poll, $(($mod_name::$F; $F, {
                        Poll::Ready(output) => match output {
                            Ok(output) => {
                                this.output.$R = MaybeUninit::new(output);
                                this.output_states[$mod_name::$F].set_ready();
                                *this.completed += 1;
                                continue;
                            },
                            Err(err) => {
                                *this.done = true;
                                *this.completed += 1;
                                return Poll::Ready(Err(err));
                            },
                        },
                        _ => continue,
                    }))*);
                }

                let all_completed = *this.completed == $mod_name::LEN;
                if all_completed {
                    // mark all error states as consumed before we return it
                    this.output_states.set_all_completed();

                    let mut output = Default::default();
                    mem::swap(&mut output, this.output);

                    *this.done = true;

                    return Poll::Ready(Ok(( $(unsafe { output.$R.assume_init() }, )+ )));
                }

                Poll::Pending
            }
        }

        #[pinned_drop]
        impl<$($F, $R,)+> PinnedDrop for $StructName<$($F, $R,)+>
        {
            fn drop(self: Pin<&mut Self>) {
                let this = self.project();

                $(
                    let mut st = this.output_states[$mod_name::$F];
                    if st.is_ready() {
                        // SAFETY: we've just filtered down to *only* the initialized values.
                        unsafe { this.output.$R.assume_init_drop() };
                        st.set_consumed();
                    }
                )+
            }
        }
    }
}

impl_try_join_tuple! { try_join_1 TryJoin1 (A ResA) }
impl_try_join_tuple! { try_join_2 TryJoin2 (A ResA) (B ResB) }
impl_try_join_tuple! { try_join_3 TryJoin3 (A ResA) (B ResB) (C ResC) }
impl_try_join_tuple! { try_join_4 TryJoin4 (A ResA) (B ResB) (C ResC) (D ResD) }
impl_try_join_tuple! { try_join_5 TryJoin5 (A ResA) (B ResB) (C ResC) (D ResD) (E ResE) }
impl_try_join_tuple! { try_join_6 TryJoin6 (A ResA) (B ResB) (C ResC) (D ResD) (E ResE) (F ResF) }
impl_try_join_tuple! { try_join_7 TryJoin7 (A ResA) (B ResB) (C ResC) (D ResD) (E ResE) (F ResF) (G ResG) }
impl_try_join_tuple! { try_join_8 TryJoin8 (A ResA) (B ResB) (C ResC) (D ResD) (E ResE) (F ResF) (G ResG) (H ResH) }
impl_try_join_tuple! { try_join_9 TryJoin9 (A ResA) (B ResB) (C ResC) (D ResD) (E ResE) (F ResF) (G ResG) (H ResH) (I ResI) }
impl_try_join_tuple! { try_join_10 TryJoin10 (A ResA) (B ResB) (C ResC) (D ResD) (E ResE) (F ResF) (G ResG) (H ResH) (I ResI) (J ResJ) }
impl_try_join_tuple! { try_join_11 TryJoin11 (A ResA) (B ResB) (C ResC) (D ResD) (E ResE) (F ResF) (G ResG) (H ResH) (I ResI) (J ResJ) (K ResK) }
impl_try_join_tuple! { try_join_12 TryJoin12 (A ResA) (B ResB) (C ResC) (D ResD) (E ResE) (F ResF) (G ResG) (H ResH) (I ResI) (J ResJ) (K ResK) (L ResL) }

#[cfg(test)]
mod test {
    use super::*;

    use std::convert::Infallible;
    use std::future;
    use std::io::{self, Error, ErrorKind};

    #[test]
    fn all_ok() {
        futures_lite::future::block_on(async {
            let a = async { Ok::<_, Infallible>("aaaa") };
            let b = async { Ok::<_, Infallible>(1) };
            let c = async { Ok::<_, Infallible>('z') };

            let result = (a, b, c).try_join().await;
            assert_eq!(result, Ok(("aaaa", 1, 'z')));
        })
    }

    #[test]
    fn one_err() {
        futures_lite::future::block_on(async {
            let err = Error::new(ErrorKind::Other, "oh no");
            let res: io::Result<(_, char)> = (future::ready(Ok("hello")), future::ready(Err(err)))
                .try_join()
                .await;
            assert_eq!(res.unwrap_err().to_string(), String::from("oh no"));
        })
    }

    #[test]
    fn issue_135_resume_after_completion() {
        use futures_lite::future::yield_now;
        futures_lite::future::block_on(async {
            let ok = async { Ok::<_, ()>(()) };
            let err = async {
                yield_now().await;
                Ok::<_, ()>(())
            };

            let res = (ok, err).try_join().await;

            assert_eq!(res.unwrap(), ((), ()));
        });
    }
}
