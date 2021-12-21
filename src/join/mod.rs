use core::future::Future;
use core::pin::Pin;

pub(crate) mod array;
pub(crate) mod tuple;
pub(crate) mod vec;

/// Wait for multiple futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the futures
/// once both complete.
pub trait Join {
    /// The resulting output type.
    type Output;
    /// The resulting joined future.
    type Future: Future<Output = Self::Output>;

    /// Waits for multiple futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the futures once both complete.
    ///
    /// This function returns a new future which polls both futures concurrently.
    fn join(self) -> Self::Future;
}

pub(crate) fn iter_pin_mut<T>(slice: Pin<&mut [T]>) -> impl Iterator<Item = Pin<&mut T>> {
    // Safety: `std` _could_ make this unsound if it were to decide Pin's
    // invariants aren't required to transmit through slices. Otherwise this has
    // the same safety as a normal field pin projection.
    unsafe { slice.get_unchecked_mut() }
        .iter_mut()
        .map(|t| unsafe { Pin::new_unchecked(t) })
}
