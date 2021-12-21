use futures_core::Stream;

pub(crate) mod array;
pub(crate) mod tuple;
pub(crate) mod vec;

/// Merge multiple streams into a single stream.
pub trait Merge {
    /// The resulting output type.
    type Item;

    /// The stream type.
    type Stream: Stream<Item = Self::Item>;

    /// Combine multiple streams into a single stream.
    fn merge(self) -> Self::Stream;
}
