use super::super::common::{CombineTuple, TupleMaybeReturn, TupleWhenCompleted};
use super::error::AggregateError;
use super::{RaceOk as RaceOkTrait, RaceOkBehavior};

use core::future::IntoFuture;
use core::marker::PhantomData;
use std::{error::Error, fmt::Display};

impl<T, E, AggE> TupleMaybeReturn<Result<T, E>, Result<T, AggE>> for RaceOkBehavior {
    type StoredItem = E;
    fn maybe_return(_: usize, res: Result<T, E>) -> Result<Self::StoredItem, Result<T, AggE>> {
        match res {
            // If subfuture returns Ok we want to early return from the combinator.
            // We do this by returning Err to the combinator.
            Ok(t) => Err(Ok(t)),
            // If subfuture returns Err, we keep the error for potential use in AggregateError.
            Err(e) => Ok(e),
        }
    }
}
impl<T, AggE> TupleWhenCompleted<AggE, Result<T, AggregateError<AggE>>> for RaceOkBehavior {
    // If we get here, it must have been that none of the subfutures early returned.
    // This means all of them failed. In this case we returned an AggregateError with the errors we kept.
    fn when_completed(errors: AggE) -> Result<T, AggregateError<AggE>> {
        Err(AggregateError { errors })
    }
}

macro_rules! impl_race_ok_tuple {
    ($(($F:ident $E:ident))+) => {
        impl<T, $($E,)+ $($F),+> RaceOkTrait for ($($F,)+)
        where $(
            $F: IntoFuture<Output = Result<T, $E>>,
        )+ {
            type Ok = T;
            type Error = AggregateError<($($E, )+)>;
            type Future = <(($($F::IntoFuture,)+), RaceOkBehavior, PhantomData<Result<T, AggregateError<($($E,)+)>>>) as CombineTuple>::Combined;
            fn race_ok(self) -> Self::Future {
                let ($($F,)+) = self;
                (
                    (
                        $($F.into_future(),)+
                    ),
                    RaceOkBehavior,
                    PhantomData
                ).combine()
            }
        }
        impl<$($E: Display),+> Display for AggregateError<($($E,)+)> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "multiple errors occurred: [")?;
                let ($($E,)+) = &self.errors;
                $(
                    write!(f, "{}", $E)?;
                )+
                write!(f, "]")
            }
        }

        impl<$($E: Error),+> Error for AggregateError<($($E,)+)> {}
    };
}

impl_race_ok_tuple! { (A0 E0) }
impl_race_ok_tuple! { (A0 E0) (A1 E1) }
impl_race_ok_tuple! { (A0 E0) (A1 E1) (A2 E2) }
impl_race_ok_tuple! { (A0 E0) (A1 E1) (A2 E2) (A3 E3) }
impl_race_ok_tuple! { (A0 E0) (A1 E1) (A2 E2) (A3 E3) (A4 E4) }
impl_race_ok_tuple! { (A0 E0) (A1 E1) (A2 E2) (A3 E3) (A4 E4) (A5 E5) }
impl_race_ok_tuple! { (A0 E0) (A1 E1) (A2 E2) (A3 E3) (A4 E4) (A5 E5) (A6 E6) }
impl_race_ok_tuple! { (A0 E0) (A1 E1) (A2 E2) (A3 E3) (A4 E4) (A5 E5) (A6 E6) (A7 E7) }
impl_race_ok_tuple! { (A0 E0) (A1 E1) (A2 E2) (A3 E3) (A4 E4) (A5 E5) (A6 E6) (A7 E7) (A8 E8) }
impl_race_ok_tuple! { (A0 E0) (A1 E1) (A2 E2) (A3 E3) (A4 E4) (A5 E5) (A6 E6) (A7 E7) (A8 E8) (A9 E9) }
impl_race_ok_tuple! { (A0 E0) (A1 E1) (A2 E2) (A3 E3) (A4 E4) (A5 E5) (A6 E6) (A7 E7) (A8 E8) (A9 E9) (A10 E10) }

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
            assert!(matches!(res.unwrap(), "world"));
        });
    }

    #[test]
    fn race_ok_2() {
        futures_lite::future::block_on(async {
            let a = future::pending::<Result<&str, ()>>();
            let b = async { Ok::<_, DynError>("world") };
            let res = (a, b).race_ok().await;
            assert!(matches!(res.unwrap(), "world"));
        });
    }

    #[test]
    fn race_ok_3() {
        futures_lite::future::block_on(async {
            let a = future::pending::<Result<&str, ()>>();
            let b = async { Ok::<_, DynError>("hello") };
            let c = async { Ok::<_, DynError>("world") };
            let result = (a, b, c).race_ok().await;
            assert!(matches!(result.unwrap(), "hello" | "world"));
        });
    }

    #[test]
    fn race_ok_err() {
        futures_lite::future::block_on(async {
            let a = async { Err::<(), _>("hello") };
            let b = async { Err::<(), _>("world") };
            let AggregateError { errors } = (a, b).race_ok().await.unwrap_err();
            assert_eq!(errors.0, "hello");
            assert_eq!(errors.1, "world");
        });
    }
}
