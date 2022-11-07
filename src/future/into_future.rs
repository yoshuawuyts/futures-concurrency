use core::future::Future;

use crate::utils::MaybeDone;

/// Conversion into a `Future`.
pub trait IntoFuture {
    /// The output that the future will produce on completion.
    type Output;
    /// Which kind of future are we turning this into?
    type IntoFuture: Future<Output = Self::Output>;
    /// Creates a future from a value.
    fn into_future(self) -> Self::IntoFuture;
}

/// TODO: in the std impl, make the bound `Fut: IntoFuture`.
impl<Fut: Future> IntoFuture for Vec<Fut> {
    type Output = Vec<Fut::Output>;

    type IntoFuture = crate::future::join::vec::Join<Fut>;

    fn into_future(self) -> Self::IntoFuture {
        use crate::future::join::vec::Join;
        Join::new(self.into_iter().collect())
    }
}

/// TODO: in the std impl, make the bound `Fut: IntoFuture`.
impl<Fut: Future, const N: usize> IntoFuture for [Fut; N] {
    type Output = [Fut::Output; N];

    type IntoFuture = crate::future::join::array::Join<Fut, N>;

    fn into_future(self) -> Self::IntoFuture {
        crate::future::join::array::Join {
            elems: self.map(|fut| MaybeDone::new(core::future::IntoFuture::into_future(fut))),
        }
    }
}
