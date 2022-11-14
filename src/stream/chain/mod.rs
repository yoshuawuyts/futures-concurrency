use futures_core::Stream;

mod array;
mod tuple;
mod vec;

/// Takes two streams and creates a new stream over both in sequence.
pub trait Chain {
    /// What's the return type of our stream?
    type Item;

    /// What stream do we return?
    type Stream: Stream<Item = Self::Item>;

    /// Combine multiple streams into a single stream.
    fn chain(self) -> Self::Stream;
}
