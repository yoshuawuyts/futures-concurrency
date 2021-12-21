use crate::stream::IntoStream;

pub(crate) mod array;
pub(crate) mod vec;

/// Merge multiple streams into a single stream.
pub trait Merge {
    /// The resulting output type.
    type Item;

    /// The stream type.
    type IntoStream: IntoStream<Item = Self::Item>;

    /// Combine multiple streams into a single stream.
    fn merge(self) -> Self::IntoStream;
}
