//! Composable asynchronous iteration.
//!
//! # Examples
//!
//! Merge multiple streams to handle values as soon as they're ready, without
//! ever dropping a single value:
//!
//! ```
//! use futures_concurrency::prelude::*;
//! use futures_concurrency::stream;
//! use futures_lite::future::block_on;
//!
//! fn main() {
//!     block_on(async {
//!         let a = stream::once(1);
//!         let b = stream::once(2);
//!         let c = stream::once(3);
//!         let s = (a, b, c).merge();
//!
//!         let mut counter = 0;
//!         s.for_each(|n| counter += n).await;
//!         assert_eq!(counter, 6);
//!     })
//! }
//! ```
//!
//! # Concurrency
//!
//! When working with multiple (async) iterators, the ordering in which
//! iterators are awaited is important. As part of async iterators, Rust
//! provides built-in operations to control the order of execution of sets of
//! iterators:
//!
//! - `merge`: combine multiple iterators into a single iterator, where the new
//! iterator yields an item as soon as one is available from one of the
//! underlying iterators.
//! - `zip`: combine multiple iterators into an iterator of pairs. The
//! underlying iterators will be awaited concurrently.
//! - `chain`: iterate over multiple iterators in sequence. The next iterator in
//! the sequence won't start until the previous iterator has finished.
//!
//! ## Futures
//!
//! Futures can be thought of as async sequences of single items. Using
//! `stream::once`, futures can be converted into async iterators and then used
//! with any of the iterator concurrency methods. This enables operations such
//! as `stream::Merge` to be used to execute sets of futures concurrently, but
//! obtain the invididual future's outputs as soon as they're available.
//!
//! See the [future concurrency][crate::future#concurrency] documentation for
//! more on futures concurrency.
pub use into_stream::IntoStream;
pub use merge::Merge;
pub use stream::Stream;

mod into_stream;
pub(crate) mod merge;
mod stream;

/// Creates a stream that yields a single item.
pub fn once<T>(t: T) -> Once<T> {
    Once { value: Some(t) }
}

#[pin_project::pin_project]
/// Stream for the [`once()`] function.
#[derive(Clone, Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct Once<T> {
    value: Option<T>,
}

impl<T> Stream for Once<T> {
    type Item = T;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<T>> {
        std::task::Poll::Ready(self.project().value.take())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.value.is_some() {
            (1, Some(1))
        } else {
            (0, Some(0))
        }
    }
}
