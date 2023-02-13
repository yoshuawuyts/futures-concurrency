use super::super::common::{CombineTuple, TupleMaybeReturn, TupleWhenCompleted};
use super::{TryJoin as TryJoinTrait, TryJoinBehavior};

use core::future::IntoFuture;
use core::marker::PhantomData;
use core::ops::ControlFlow;

use futures_core::TryFuture;

impl<T, AggT, E> TupleMaybeReturn<Result<T, E>, Result<AggT, E>> for TryJoinBehavior {
    type StoredItem = T;
    fn maybe_return(_: usize, res: Result<T, E>) -> ControlFlow<Result<AggT, E>, Self::StoredItem> {
        match res {
            Ok(t) => ControlFlow::Continue(t),
            Err(e) => ControlFlow::Break(Err(e)),
        }
    }
}
impl<AggT, E> TupleWhenCompleted<AggT, Result<AggT, E>> for TryJoinBehavior {
    fn when_completed(stored_items: AggT) -> Result<AggT, E> {
        Ok(stored_items)
    }
}

macro_rules! impl_try_join_tuple {
    ($(($F:ident $T:ident))+) => {
        impl<E, $($T,)+ $($F),+> TryJoinTrait for ($($F,)+)
        where $(
            $F: IntoFuture<Output = Result<$T, E>>,
        )+ {
            type Ok = ($(<$F::IntoFuture as TryFuture>::Ok,)+);
            type Error = E;
            type Future = <(($($F::IntoFuture,)+), TryJoinBehavior, PhantomData<Result<($($T,)+), E>>) as CombineTuple>::Combined;
            fn try_join(self) -> Self::Future {
                let ($($F,)+) = self;
                (
                    ($($F.into_future(),)+),
                    TryJoinBehavior,
                    PhantomData
                ).combine()
            }
        }
    };
}

impl_try_join_tuple! { (A0 T0) }
impl_try_join_tuple! { (A0 T0) (A1 T1) }
impl_try_join_tuple! { (A0 T0) (A1 T1) (A2 T2) }
impl_try_join_tuple! { (A0 T0) (A1 T1) (A2 T2) (A3 T3) }
impl_try_join_tuple! { (A0 T0) (A1 T1) (A2 T2) (A3 T3) (A4 T4) }
impl_try_join_tuple! { (A0 T0) (A1 T1) (A2 T2) (A3 T3) (A4 T4) (A5 T5) }
impl_try_join_tuple! { (A0 T0) (A1 T1) (A2 T2) (A3 T3) (A4 T4) (A5 T5) (A6 T6) }
impl_try_join_tuple! { (A0 T0) (A1 T1) (A2 T2) (A3 T3) (A4 T4) (A5 T5) (A6 T6) (A7 T7) }
impl_try_join_tuple! { (A0 T0) (A1 T1) (A2 T2) (A3 T3) (A4 T4) (A5 T5) (A6 T6) (A7 T7) (A8 T8) }
impl_try_join_tuple! { (A0 T0) (A1 T1) (A2 T2) (A3 T3) (A4 T4) (A5 T5) (A6 T6) (A7 T7) (A8 T8) (A9 T9) }
impl_try_join_tuple! { (A0 T0) (A1 T1) (A2 T2) (A3 T3) (A4 T4) (A5 T5) (A6 T6) (A7 T7) (A8 T8) (A9 T9) (A10 T10) }

#[cfg(test)]
mod test {
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
            assert_eq!(res.unwrap_err().kind(), ErrorKind::Other);
        });
    }
}
