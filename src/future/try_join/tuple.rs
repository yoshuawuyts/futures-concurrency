use super::super::common::{CombineTuple, MapResult};
use super::TryJoin as TryJoinTrait;

use core::fmt::Debug;
use core::future::IntoFuture;
use futures_core::TryFuture;

#[derive(Debug)]
pub struct MapResultTryJoin;
impl<T, E> MapResult<Result<T, E>> for MapResultTryJoin {
    type FinalResult = Result<T, E>;
    fn to_final_result(result: Result<T, E>) -> Self::FinalResult {
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
                let ($($F,)+) = self;
                (
                    ($($F.into_future(),)+),
                    MapResultTryJoin
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

#[cfg(test)]
mod test {
    use crate::future::select_types;

    use super::*;
    use std::future;
    use std::io::{self, Error, ErrorKind};

    #[test]
    fn ok() {
        futures_lite::future::block_on(async {
            let res = (
                future::ready(Result::<_, io::Error>::Ok(42)),
                future::ready(Result::<_, io::Error>::Ok("world")),
            )
                .try_join()
                .await;
            assert_eq!(res.unwrap(), (42, "world"));
        })
    }

    #[test]
    fn err() {
        futures_lite::future::block_on(async {
            let err = Error::new(ErrorKind::Other, "oh no");
            let res = (
                future::ready(io::Result::Ok("hello")),
                future::ready(Result::<i32, _>::Err(err)),
            )
                .try_join()
                .await;
            match res.unwrap_err() {
                select_types::SelectedFrom2::A0(_l) => panic!("this one should be ok"),
                select_types::SelectedFrom2::A1(r) => {
                    assert_eq!(r.to_string(), "oh no".to_string())
                }
            };
        });
    }
}
