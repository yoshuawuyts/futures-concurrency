use super::super::common::{CombineTuple, MapResult};
use super::RaceOk as RaceOkTrait;

use core::fmt::Debug;
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_core::TryFuture;

#[derive(Debug)]
#[pin_project::pin_project]
pub struct RaceOkFuture<F: Future>(#[pin] F);
impl<F: TryFuture> Future for RaceOkFuture<F> {
    type Output = Result<F::Error, F::Ok>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().0.try_poll(cx).map(|res| match res {
            Ok(t) => Err(t),
            Err(e) => Ok(e),
        })
    }
}

#[derive(Debug)]
pub struct MapResultRaceOk;
impl<E, T> MapResult<Result<E, T>> for MapResultRaceOk {
    type FinalResult = Result<T, E>;
    fn to_final_result(result: Result<E, T>) -> Self::FinalResult {
        match result {
            Ok(e) => Err(e),
            Err(t) => Ok(t),
        }
    }
}

macro_rules! impl_race_ok_tuple {
    ($($F:ident)+) => {
        impl<$($F),+> RaceOkTrait for ($($F,)+)
        where $(
            $F: IntoFuture,
			$F::IntoFuture: TryFuture,
        )+ {
            type Ok = <Self::Future as TryFuture>::Ok;
            type Error = ($(<$F::IntoFuture as TryFuture>::Error,)+);
            type Future = <(($(RaceOkFuture<$F::IntoFuture>,)+), MapResultRaceOk) as CombineTuple>::Combined;
            fn race_ok(self) -> Self::Future {
                let ($($F,)+) = self;
                (
                    (
                        $(RaceOkFuture($F.into_future()),)+
                    ),
                    MapResultRaceOk
                ).combine()
            }
        }
    };
}

impl_race_ok_tuple! { A0 }
impl_race_ok_tuple! { A0 A1 }
impl_race_ok_tuple! { A0 A1 A2 }
impl_race_ok_tuple! { A0 A1 A2 A3 }
impl_race_ok_tuple! { A0 A1 A2 A3 A4 }
impl_race_ok_tuple! { A0 A1 A2 A3 A4 A5 }
impl_race_ok_tuple! { A0 A1 A2 A3 A4 A5 A6 }
impl_race_ok_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 }
impl_race_ok_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 }
impl_race_ok_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 A9 }
impl_race_ok_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 }

#[cfg(test)]
mod test {
    use super::*;
    use core::future;
    use std::error::Error;

    type DynError = Box<dyn Error>;

    #[test]
    fn race_ok_1() {
        futures_lite::future::block_on(async {
            let a = async { Ok::<_, DynError>("world") };
            let res = (a,).race_ok().await;
            assert!(matches!(res.unwrap().any(), "world"));
        });
    }

    #[test]
    fn race_ok_2() {
        futures_lite::future::block_on(async {
            let a = future::pending::<Result<&str, ()>>();
            let b = async { Ok::<_, DynError>("world") };
            let res = (a, b).race_ok().await;
            assert!(matches!(res.unwrap().any(), "world"));
        });
    }

    #[test]
    fn race_ok_3() {
        futures_lite::future::block_on(async {
            let a = future::pending::<Result<&str, ()>>();
            let b = async { Ok::<_, DynError>("hello") };
            let c = async { Ok::<_, DynError>("world") };
            let result = (a, b, c).race_ok().await;
            assert!(matches!(result.unwrap().any(), "hello" | "world"));
        });
    }

    #[test]
    fn race_ok_err() {
        futures_lite::future::block_on(async {
            let a = async { Err::<(), _>("hello") };
            let b = async { Err::<(), _>("world") };
            let errors = (a, b).race_ok().await.unwrap_err();
            assert_eq!(errors.0, "hello");
            assert_eq!(errors.1, "world");
        });
    }
}
