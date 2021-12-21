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
    ///         let c = stream::once(3u8);
    ///         let s = a.merge(b).merge(c);
    ///
    ///         let mut buf = vec![];
    ///         s.for_each(|n| buf.push(n)).await;
    ///         buf.sort_unstable();
    ///         assert_eq!(&buf, &[1u8, 2u8, 3u8]);
    ///     })
    /// }
    /// ```
    fn merge<U>(self, other: U) -> Merge<Self, U>
    where
        Self: Sized,
        U: Stream<Item = Self::Item> + Sized,
    {
        Merge::new(self, other)
    }
}

impl<S> StreamExt for S where S: Stream {}
