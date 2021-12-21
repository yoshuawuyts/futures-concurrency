//! Composable asynchronous iteration.
use crate::array::Merge;

use futures_core::Stream;

/// Extend `Stream` with concurrency methods.
pub trait StreamExt: Stream {
    /// Combines multiple streams into a single stream of all their outputs.
    ///
    /// Items are yielded as soon as they're received, and the stream continues
    /// yield until both streams have been exhausted. The output ordering
    /// between streams is not guaranteed.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_lite::prelude::*;
    /// use futures_lite::future::block_on;
    /// use futures_lite::stream;
    /// use futures_concurrency::prelude::*;
    ///
    /// fn main() {
    ///     block_on(async {
    ///         let a = stream::once(1u8);
    ///         let b = stream::once(2u8);
    ///         let s = a.merge(b);
    ///
    ///         let mut buf = vec![];
    ///         s.for_each(|n| buf.push(n)).await;
    ///         buf.sort_unstable();
    ///         assert_eq!(&buf, &[1u8, 2u8]);
    ///     })
    /// }
    /// ```
    fn merge(self, other: Self) -> Merge<Self, 2>
    where
        Self: Sized,
    {
        Merge::new([self, other])
    }
}

impl<S> StreamExt for S where S: Stream {}

/// Conversion into a `Stream`.
///
/// By implementing `IntoIterator` for a type, you define how it will be
/// converted to an iterator. This is common for types which describe a
/// collection of some kind.
///
/// [`from_stream`]: #tymethod.from_stream
/// [`Stream`]: trait.Stream.html
/// [`collect`]: trait.Stream.html#method.collect
///
/// See also: [`FromStream`].
///
/// [`FromStream`]: trait.FromStream.html
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
