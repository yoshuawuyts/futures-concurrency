use super::super::common::{CombinatorArray, CombinatorBehaviorArray};
use super::{RaceOk as RaceOkTrait, RaceOkBehavior};

use core::future::{Future, IntoFuture};

/// Wait for the first successful future to complete.
///
/// This `struct` is created by the [`race_ok`] method on the [`RaceOk`] trait. See
/// its documentation for more.
///
/// [`race_ok`]: crate::future::RaceOk::race_ok
/// [`RaceOk`]: crate::future::RaceOk
pub type RaceOk<Fut, const N: usize> = CombinatorArray<Fut, RaceOkBehavior, N>;

impl<T, E, Fut, const N: usize> CombinatorBehaviorArray<Fut, N> for RaceOkBehavior
where
    Fut: Future<Output = Result<T, E>>,
{
    type Output = Result<T, [E; N]>;

    type StoredItem = E;

    fn maybe_return(
        _idx: usize,
        res: <Fut as Future>::Output,
    ) -> Result<Self::StoredItem, Self::Output> {
        match res {
            Ok(v) => Err(Ok(v)),
            Err(e) => Ok(e),
        }
    }

    fn when_completed_arr(arr: [Self::StoredItem; N]) -> Self::Output {
        Err(arr)
    }
}

impl<T, E, Fut, const N: usize> RaceOkTrait for [Fut; N]
where
    Fut: IntoFuture,
    Fut::IntoFuture: Future<Output = Result<T, E>>,
{
    type Ok = T;
    type Error = [E; N];
    type Future = RaceOk<Fut::IntoFuture, N>;

    fn race_ok(self) -> Self::Future {
        RaceOk::new(self.map(IntoFuture::into_future))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::future;
    use std::io::{Error, ErrorKind};

    #[test]
    fn all_ok() {
        futures_lite::future::block_on(async {
            let res: Result<&str, [Error; 2]> =
                [future::ready(Ok("hello")), future::ready(Ok("world"))]
                    .race_ok()
                    .await;
            assert!(res.is_ok());
        })
    }

    #[test]
    fn one_err() {
        futures_lite::future::block_on(async {
            let err = Error::new(ErrorKind::Other, "oh no");
            let res: Result<&str, [Error; 2]> =
                [future::ready(Ok("hello")), future::ready(Err(err))]
                    .race_ok()
                    .await;
            assert_eq!(res.unwrap(), "hello");
        });
    }

    #[test]
    fn all_err() {
        futures_lite::future::block_on(async {
            let err1 = Error::new(ErrorKind::Other, "oops");
            let err2 = Error::new(ErrorKind::Other, "oh no");
            let res: Result<&str, [Error; 2]> =
                [future::ready(Err(err1)), future::ready(Err(err2))]
                    .race_ok()
                    .await;
            let errs = res.unwrap_err();
            assert_eq!(errs[0].to_string(), "oops");
            assert_eq!(errs[1].to_string(), "oh no");
        });
    }
}
