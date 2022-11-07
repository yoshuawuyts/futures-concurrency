use core::future::Future;

pub(crate) mod array;
pub(crate) mod tuple;
pub(crate) mod vec;

/// Wait for the first future to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the first
/// future which completes.
pub trait Race {
    /// The resulting output type.
    type Output;

    /// Which kind of future are we turning this into?
    type Future: Future<Output = Self::Output>;

    /// Waits for multiple futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    ///
    /// This function returns a new future which polls both futures
    /// concurrently.
    fn race(self) -> Self::Future;
}
