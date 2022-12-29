use super::TryJoin as TryJoinTrait;
use super::super::common::{CombineTuple, MapResult,};

use core::fmt::{Debug};
use core::future::{Future, IntoFuture};
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::TryFuture;

#[derive(Debug)]
pub struct MapResultTryJoin;
impl<T, E> MapResult<Result<T, E>> for MapResultTryJoin {
    type FinalResult = Result<T, E>;
    fn to_final_result(result: Result<E, T>) -> Self::FinalResult {
        result
    }
}


macro_rules! impl_try_join_tuple {
    ($($F:ident)+) => {
        impl<$($F),+> TryJoinTrait for ($($F,)+)
        where $(
            $F: IntoFuture,
			$F::IntoFuture: TryFuture,
        )+ {
            type Ok = ($(<$F::IntoFuture as TryFuture>::Ok,)+);
            type Error = <Self::Future as TryFuture>::Error;
            type Future = <(($($F::IntoFuture,)+), MapResultTryJoin) as CombineTuple>::Combined;
            fn try_join(self) -> Self::Future {
                (
                    self,
                    MapResultRaceOk
                ).combine()
            }
        }
    };
}

impl_try_join_tuple! { A0 }
impl_try_join_tuple! { A0 A1 }
impl_try_join_tuple! { A0 A1 A2 }
impl_try_join_tuple! { A0 A1 A2 A3 }
impl_try_join_tuple! { A0 A1 A2 A3 A4 }
impl_try_join_tuple! { A0 A1 A2 A3 A4 A5 }
impl_try_join_tuple! { A0 A1 A2 A3 A4 A5 A6 }
impl_try_join_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 }
impl_try_join_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 }
impl_try_join_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 A9 }
impl_try_join_tuple! { A0 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 }