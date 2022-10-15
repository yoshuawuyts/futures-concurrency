use crate::stream::Stream;

use super::Merge;

/// Conversion into a `Stream`.
///
/// By implementing `IntoStream` for a type, you define how it will be
/// converted to an iterator. This is common for types which describe a
/// collection of some kind.
///
/// [`from_stream`]: #tymethod.from_stream
/// [`Stream`]: trait.Stream.html
pub trait IntoStream {
    /// The type of the elements being iterated over.
    type Item;

    /// Which kind of stream are we turning this into?
    type IntoStream: Stream<Item = Self::Item>;

    /// Creates a stream from a value.
    fn into_stream(self) -> Self::IntoStream;
}

impl<S: Stream> IntoStream for S {
    type Item = S::Item;
    type IntoStream = S;

    #[inline]
    fn into_stream(self) -> S {
        self
    }
}

impl<S: IntoStream> IntoStream for Vec<S> {
    type Item = <crate::vec::Merge<S::IntoStream> as Stream>::Item;
    type IntoStream = crate::vec::Merge<S::IntoStream>;

    fn into_stream(self) -> Self::IntoStream {
        self.merge()
    }
}

impl<S: IntoStream, const N: usize> IntoStream for [S; N] {
    type Item = <crate::array::Merge<S::IntoStream, N> as Stream>::Item;
    type IntoStream = crate::array::Merge<S::IntoStream, N>;

    fn into_stream(self) -> Self::IntoStream {
        self.merge()
    }
}
