use super::super::common::{CombinatorArray, CombinatorBehaviorArray};
use super::{Race as RaceTrait, RaceBehavior};

use core::future::{Future, IntoFuture};
use core::ops::ControlFlow;

/// Wait for the first future to complete.
///
/// This `struct` is created by the [`race`] method on the [`Race`] trait. See
/// its documentation for more.
///
/// [`race`]: crate::future::Race::race
/// [`Race`]: crate::future::Race
pub type Race<Fut, const N: usize> = CombinatorArray<Fut, RaceBehavior, N>;

impl<Fut, const N: usize> CombinatorBehaviorArray<Fut, N> for RaceBehavior
where
    Fut: Future,
{
    type Output = Fut::Output;

    type StoredItem = core::convert::Infallible;

    fn maybe_return(
        _idx: usize,
        res: <Fut as Future>::Output,
    ) -> ControlFlow<Self::Output, Self::StoredItem> {
        // Subfuture finished, so the race is over. Break now.
        ControlFlow::Break(res)
    }

    fn when_completed(_arr: [Self::StoredItem; N]) -> Self::Output {
        panic!("race only works on non-empty arrays");
    }
}

impl<Fut, const N: usize> RaceTrait for [Fut; N]
where
    Fut: IntoFuture,
{
    type Output = Fut::Output;
    type Future = Race<Fut::IntoFuture, N>;

    fn race(self) -> Self::Future {
        Race::new(self.map(IntoFuture::into_future))
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
            let res = [future::ready("hello"), future::ready("world")]
                .race()
                .await;
            assert!(matches!(res, "hello" | "world"));
        });
    }
}
