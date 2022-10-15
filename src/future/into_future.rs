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

    type IntoFuture = crate::future::merge::vec::Merge<Fut>;

    fn into_future(self) -> Self::IntoFuture {
        let elems = self
            .into_iter()
            .map(|fut| MaybeDone::new(core::future::IntoFuture::into_future(fut)))
            .collect::<Box<_>>()
            .into();
        crate::future::merge::vec::Merge::new(elems)
    }
}

/// TODO: in the std impl, make the bound `Fut: IntoFuture`.
impl<Fut: Future, const N: usize> IntoFuture for [Fut; N] {
    type Output = [Fut::Output; N];

    type IntoFuture = crate::future::merge::array::Merge<Fut, N>;

    fn into_future(self) -> Self::IntoFuture {
        crate::future::merge::array::Merge {
            elems: self.map(|fut| MaybeDone::new(core::future::IntoFuture::into_future(fut))),
        }
    }
}
