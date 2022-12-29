use super::super::common::{CombineTuple, MapResult};
use super::Race as RaceTrait;

use core::fmt::Debug;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

#[derive(Debug)]
#[pin_project::pin_project]
pub struct RaceFuture<F: Future>(#[pin] F);
impl<F: Future> Future for RaceFuture<F> {
    type Output = Result<core::convert::Infallible, F::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().0.poll(cx).map(Err)
    }
}

#[derive(Debug)]
pub struct MapResultRace;
impl<R, S> MapResult<Result<R, S>> for MapResultRace {
    type FinalResult = S;
    fn to_final_result(result: Result<R, S>) -> Self::FinalResult {
        match result {
            Err(s) => s,
            Ok(_r) => unreachable!(),
        }
    }
}

macro_rules! impl_race_tuple {
    ($($F:ident)+) => {
        impl<$($F),+> RaceTrait for ($($F,)+)
        where $(
            $F: IntoFuture,
        )+ {
            type Output = <Self::Future as Future>::Output;
            type Future = <(($(RaceFuture<$F::IntoFuture>,)+), MapResultRace) as CombineTuple>::Combined;
            fn race(self) -> Self::Future {
                let ($($F,)+) = self;
                (
                    (
                        $(RaceFuture($F.into_future()),)+
                    ),
                    MapResultRace
                ).combine()
            }
        }
    };
}

impl_race_tuple! { A0 }
impl_race_tuple! { A0 A1 }
impl_race_tuple! { A0 A1 A2 }
impl_race_tuple! { A0 A1 A2 A3 }
impl_race_tuple! { A0 A1 A2 A3 A4 }
impl_race_tuple! { A0 A1 A2 A3 A4 A5 }
impl_race_tuple! { A0 A1 A2 A3 A4 A5 A6 }
impl_race_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 }
impl_race_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 }
impl_race_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 A9 }
impl_race_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 }
impl_race_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 }

#[cfg(test)]
mod test {
    use super::*;
    use std::future;

    #[test]
    fn race_1() {
        futures_lite::future::block_on(async {
            let a = future::ready("world");
            assert_eq!((a,).race().await.any(), "world");
        });
    }

    #[test]
    fn race_2() {
        futures_lite::future::block_on(async {
            let a = future::pending();
            let b = future::ready("world");
            assert_eq!((a, b).race().await.any(), "world");
        });
    }

    #[test]
    fn race_3() {
        futures_lite::future::block_on(async {
            let a = future::pending();
            let b = future::ready("hello");
            let c = future::ready("world");
            let result = (a, b, c).race().await.any();
            assert!(matches!(result, "hello" | "world"));
        });
    }
}
