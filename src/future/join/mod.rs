use core::future::Future;

use super::common::{CombinatorBehaviorArray, CombinatorBehaviorVec, ReturnOrStore};

pub(crate) mod array;
pub(crate) mod tuple;
pub(crate) mod vec;

/// Wait for all futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the futures
/// once all complete.
pub trait Join {
    /// The resulting output type.
    type Output;

    /// Which kind of future are we turning this into?
    type Future: Future<Output = Self::Output>;

    /// Waits for multiple futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once all complete.
    ///
    /// This function returns a new future which polls all futures concurrently.
    fn join(self) -> Self::Future;
}

#[derive(Debug)]
pub struct JoinBehavior;
impl<Fut, const N: usize> CombinatorBehaviorArray<Fut, N> for JoinBehavior
where
    Fut: Future,
{
    type Output = [Fut::Output; N];

    type StoredItem = Fut::Output;

    fn maybe_return(
        _idx: usize,
        res: <Fut as Future>::Output,
    ) -> ReturnOrStore<Self::Output, Self::StoredItem> {
        ReturnOrStore::Store(res)
    }

    fn when_completed_arr(arr: [Self::StoredItem; N]) -> Self::Output {
        arr
    }
}

impl<Fut> CombinatorBehaviorVec<Fut> for JoinBehavior
where
    Fut: Future,
{
    type Output = Vec<Fut::Output>;

    type StoredItem = Fut::Output;

    fn maybe_return(
        _idx: usize,
        res: <Fut as Future>::Output,
    ) -> ReturnOrStore<Self::Output, Self::StoredItem> {
        ReturnOrStore::Store(res)
    }

    fn when_completed_vec(vec: Vec<Self::StoredItem>) -> Self::Output {
        vec
    }
}
