use super::super::common::{CombinatorBehaviorVec, CombinatorVec};
use super::{Race as RaceTrait, RaceBehavior};
use core::future::{Future, IntoFuture};

/// Wait for the first future to complete.
///
/// This `struct` is created by the [`race`] method on the [`Race`] trait. See
/// its documentation for more.
///
/// [`race`]: crate::future::Race::race
/// [`Race`]: crate::future::Race
pub type Race<Fut> = CombinatorVec<Fut, RaceBehavior>;

impl<Fut> CombinatorBehaviorVec<Fut> for RaceBehavior
where
    Fut: Future,
{
    type Output = Fut::Output;

    type StoredItem = core::convert::Infallible;

    fn maybe_return(
        _idx: usize,
        res: <Fut as Future>::Output,
    ) -> Result<Self::StoredItem, Self::Output> {
        Err(res)
    }

    fn when_completed_vec(_vec: Vec<Self::StoredItem>) -> Self::Output {
        panic!("race only works on non-empty arrays");
    }
}

impl<Fut> RaceTrait for Vec<Fut>
where
    Fut: IntoFuture,
{
    type Output = Fut::Output;
    type Future = Race<Fut::IntoFuture>;

    fn race(self) -> Self::Future {
        Race::new(self.into_iter().map(IntoFuture::into_future).collect())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::future;

    // NOTE: we should probably poll in random order.
    #[test]
    fn no_fairness() {
        futures_lite::future::block_on(async {
            let res = vec![future::ready("hello"), future::ready("world")]
                .race()
                .await;
            assert!(matches!(res, "hello" | "world"));
        });
    }
}
