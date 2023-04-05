use crate::stream::{IntoStream, Merge};
use futures_core::Stream;

use super::merge::tuple::Merge2;

/// An extension trait for the `Stream` trait.
pub trait StreamExt: Stream {
    /// Combines two streams into a single stream of all their outputs.
    fn merge<T, S2>(self, other: S2) -> Merge2<T, Self, S2::IntoStream>
    where
        Self: Stream<Item = T> + Sized,
        S2: IntoStream<Item = T>;
}

impl<S1> StreamExt for S1
where
    S1: Stream,
{
    fn merge<T, S2>(self, other: S2) -> Merge2<T, S1, S2::IntoStream>
    where
        S1: Stream<Item = T>,
        S2: IntoStream<Item = T>,
    {
        Merge::merge((self, other))
    }
}
