use super::{Join as JoinTrait, JoinBehavior};
use crate::future::common::CombinatorVec;

use core::future::IntoFuture;
use std::vec::Vec;

/// Waits for two similarly-typed futures to complete.
///
/// This `struct` is created by the [`join`] method on the [`Join`] trait. See
/// its documentation for more.
///
/// [`join`]: crate::future::Join::join
/// [`Join`]: crate::future::Join
pub type Join<Fut> = CombinatorVec<Fut, JoinBehavior>;

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
    use crate::utils::dummy_waker;

    use super::*;

    use std::future;
    use std::future::Future;
    use std::pin::Pin;
    use std::task::Context;

    #[test]
    fn smoke() {
        futures_lite::future::block_on(async {
            let fut = vec![future::ready("hello"), future::ready("world")].join();
            assert_eq!(fut.await, vec!["hello", "world"]);
        });
    }

    #[test]
    fn debug() {
        let mut fut = vec![future::ready("hello"), future::ready("world")].join();
        assert_eq!(format!("{:?}", fut), "[Pending, Pending]");
        let mut fut = Pin::new(&mut fut);

        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);
        let _ = fut.as_mut().poll(&mut cx);
        assert_eq!(format!("{:?}", fut), "[Consumed, Consumed]");
    }
}
