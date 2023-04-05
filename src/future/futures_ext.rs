use crate::future::Join;
use crate::future::Race;
use futures_core::Future;
use std::future::IntoFuture;

use super::join::tuple::Join2;
use super::race::tuple::Race2;

/// An extension trait for the `Future` trait.
pub trait FutureExt: Future {
    /// Wait for both futures to complete.
    fn join<S2>(self, other: S2) -> Join2<Self, S2::IntoFuture>
    where
        Self: Future + Sized,
        S2: IntoFuture;

    /// Wait for the first future to complete.
    fn race<T, S2>(self, other: S2) -> Race2<T, Self, S2::IntoFuture>
    where
        Self: Future<Output = T> + Sized,
        S2: IntoFuture<Output = T>;
}

impl<F1> FutureExt for F1
where
    F1: Future,
{
    fn join<F2>(self, other: F2) -> Join2<Self, F2::IntoFuture>
    where
        Self: Future + Sized,
        F2: IntoFuture,
    {
        Join::join((self, other))
    }

    fn race<T, S2>(self, other: S2) -> Race2<T, Self, S2::IntoFuture>
    where
        Self: Future<Output = T> + Sized,
        S2: IntoFuture<Output = T>,
    {
        Race::race((self, other))
    }
}
