use super::super::common::{CombineTuple, TupleMaybeReturn, TupleWhenCompleted};
use super::{Race as RaceTrait, RaceBehavior};

use core::convert::Infallible;
use core::future::{Future, IntoFuture};
use core::marker::PhantomData;

impl<T> TupleMaybeReturn<T, T> for RaceBehavior {
    // We early return as soon as any subfuture finishes.
    // Results from subfutures are never stored.
    type StoredItem = Infallible;
    fn maybe_return(_: usize, res: T) -> Result<Self::StoredItem, T> {
        // Err = early return.
        Err(res)
    }
}
impl<S, O> TupleWhenCompleted<S, O> for RaceBehavior {
    // We always early return, so we should never get here.
    fn when_completed(_: S) -> O {
        unreachable!() // should have early returned
    }
}

macro_rules! impl_race_tuple {
    ($($F:ident)+) => {
        impl<T, $($F),+> RaceTrait for ($($F,)+)
        where $(
            $F: IntoFuture<Output = T>,
        )+ {
            type Output = <Self::Future as Future>::Output;
            type Future = <(($($F::IntoFuture,)+), RaceBehavior, PhantomData<T>) as CombineTuple>::Combined;
            fn race(self) -> Self::Future {
                let ($($F,)+) = self;
                (
                    (
                        $($F.into_future(),)+
                    ),
                    RaceBehavior,
                    PhantomData
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
            assert_eq!((a,).race().await, "world");
        });
    }

    #[test]
    fn race_2() {
        futures_lite::future::block_on(async {
            let a = future::pending();
            let b = future::ready("world");
            assert_eq!((a, b).race().await, "world");
        });
    }

    #[test]
    fn race_3() {
        futures_lite::future::block_on(async {
            let a = future::pending();
            let b = future::ready("hello");
            let c = future::ready("world");
            let result = (a, b, c).race().await;
            assert!(matches!(result, "hello" | "world"));
        });
    }
}
