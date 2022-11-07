//! Concurrency extensions for `Future` and `Stream`.
//!
//! Companion library for the "Futures Concurrency" blog post
//! series:
//! - [Futures Concurrency I: Introduction](https://blog.yoshuawuyts.com/futures-concurrency/)
//! - [Futures Concurrency II: A Trait Approach](https://blog.yoshuawuyts.com/futures-concurrency-2/)
//! - [Futures Concurrency III: `select!`](https://blog.yoshuawuyts.com/futures-concurrency-3/)
//! - [Futures Concurrency IV: Join Semantics](https://blog.yoshuawuyts.com/futures-concurrency-4/)
//!
//! The purpose of this library is to serve as a staging ground for what
//! eventually may become the futures concurrency methods provided by the
//! stdlib. See the [`future`] and [`stream`] submodules for more.
//!
//! # Limitations
//!
//! Because of orphan rules this library can't implement everything the stdlib
//! can. The missing implementations are:
//!
//! - `impl<T> IntoFuture for Vec<T>`
//! - `impl<T, const N: usize> IntoFuture for [T; N]`
//! - `impl<T..> IntoFuture for (T..)`
//! - `impl<T> IntoAsyncIterator for Vec<T>`
//! - `impl<T, const N: usize> IntoAsyncIterator for [T; N]`
//! - `impl<T..> IntoAsyncIterator for (T..)`
//!
//! This would enable containers of futures to directly be `.await`ed to get
//! `merge` semantics. Or containers of async iterators to be passed directly to
//! `for..await in` loops to be iterated over using `merge` semantics. This would
//! remove the need to think of "merge" as a verb, and would enable treating
//! sets of futures concurrently.
//!
//! # Examples
//!
//! Concurrently await multiple heterogenous futures:
//! ```rust
//! use futures_concurrency::prelude::*;
//! use futures_lite::future::block_on;
//! use std::future;
//!
//! fn main() {
//!     block_on(async {
//!         let a = future::ready(1u8);
//!         let b = future::ready("hello");
//!         let c = future::ready(3u16);
//!         assert_eq!((a, b, c).join().await, (1, "hello", 3));
//!     })
//! }
//! ```

#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(missing_docs, unreachable_pub)]
#![allow(non_snake_case)]

mod utils;

/// The futures concurrency prelude.
pub mod prelude {
    pub use super::future::Join as _;
    pub use super::future::Race as _;
    pub use super::future::RaceOk as _;
    pub use super::future::TryJoin as _;
    pub use super::stream::IntoStream as _;
    pub use super::stream::Merge as _;
    pub use super::stream::Stream as _;
}

pub mod future;
pub mod stream;

/// Helper functions and types for fixed-length arrays.
pub mod array {
    pub use crate::future::race_ok::array::AggregateError;
    pub use crate::stream::merge::array::Merge;
}
/// A contiguous growable array type with heap-allocated contents, written `Vec<T>`.
pub mod vec {
    pub use crate::future::race_ok::vec::AggregateError;
    pub use crate::stream::merge::vec::Merge;
}
