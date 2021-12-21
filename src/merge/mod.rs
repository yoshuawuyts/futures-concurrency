use crate::stream::IntoStream;

pub(crate) mod array;
pub(crate) mod vec;

/// Wait for multiple futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the futures
/// once both complete.
pub trait Merge {
    /// The resulting output type.
    type Item;

    /// The stream type.
    type IntoStream: IntoStream<Item = Self::Item>;

    /// Combine multiple streams into a single stream.
    fn merge(self) -> Self::IntoStream;
}
