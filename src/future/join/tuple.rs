use super::super::common::{CombineTuple, MapResult};
use super::Join as JoinTrait;

use core::fmt::Debug;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Debug)]
#[pin_project::pin_project]
pub struct JoinFuture<F: Future>(#[pin] F);
impl<F: Future> Future for JoinFuture<F> {
    type Output = Result<F::Output, core::convert::Infallible>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().0.poll(cx).map(Ok)
    }
}

#[derive(Debug)]
pub struct MapResultJoin;
impl<R, S> MapResult<Result<R, S>> for MapResultJoin {
    type FinalResult = R;
    fn to_final_result(result: Result<R, S>) -> Self::FinalResult {
        match result {
            Err(_s) => unreachable!(),
            Ok(r) => r,
        }
    }
}

macro_rules! impl_join_tuple {
    ($($F:ident)+) => {
        impl<$($F),+> JoinTrait for ($($F,)+)
        where $(
            $F: IntoFuture,
        )+ {
            type Output = ($($F::Output,)+);
            type Future = <(($(JoinFuture<$F::IntoFuture>,)+), MapResultJoin) as CombineTuple>::Combined;
            fn join(self) -> Self::Future {
                let ($($F,)+) = self;
                (
                    (
                        $(JoinFuture($F.into_future()),)+
                    ),
                    MapResultJoin
                ).combine()
            }
        }
    };
}

impl JoinTrait for () {
    type Output = ();
    type Future = core::future::Ready<()>;

    fn join(self) -> Self::Future {
        core::future::ready(())
    }
}

impl_join_tuple! { A0 }
impl_join_tuple! { A0 A1 }
impl_join_tuple! { A0 A1 A2 }
impl_join_tuple! { A0 A1 A2 A3 }
impl_join_tuple! { A0 A1 A2 A3 A4 }
impl_join_tuple! { A0 A1 A2 A3 A4 A5 }
impl_join_tuple! { A0 A1 A2 A3 A4 A5 A6 }
impl_join_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 }
impl_join_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 }
impl_join_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 A9 }
impl_join_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 }
impl_join_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 }

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
