use super::super::common::{CombinatorBehaviorVec, CombinatorVec};
use super::{Join as JoinTrait, JoinBehavior};

use core::future::{Future, IntoFuture};
use std::vec::Vec;

/// Waits for two similarly-typed futures to complete.
///
/// This `struct` is created by the [`join`] method on the [`Join`] trait. See
/// its documentation for more.
///
/// [`join`]: crate::future::Join::join
/// [`Join`]: crate::future::Join
pub type Join<Fut> = CombinatorVec<Fut, JoinBehavior>;

impl<Fut> CombinatorBehaviorVec<Fut> for JoinBehavior
where
    Fut: Future,
{
    type Output = Vec<Fut::Output>;

    type StoredItem = Fut::Output;

    fn maybe_return(
        _idx: usize,
        res: <Fut as Future>::Output,
    ) -> Result<Self::StoredItem, Self::Output> {
        Ok(res)
    }

    fn when_completed_vec(vec: Vec<Self::StoredItem>) -> Self::Output {
        vec
    }
}

impl<Fut> JoinTrait for Vec<Fut>
where
    Fut: IntoFuture,
{
    type Output = Vec<Fut::Output>;
    type Future = Join<Fut::IntoFuture>;

    fn join(self) -> Self::Future {
        Join::new(self.into_iter().map(IntoFuture::into_future).collect())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::future;

    #[test]
    fn smoke() {
        futures_lite::future::block_on(async {
            let fut = vec![future::ready("hello"), future::ready("world")].join();
            assert_eq!(fut.await, vec!["hello", "world"]);
        });
    }
}
