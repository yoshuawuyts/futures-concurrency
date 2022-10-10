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
#![cfg_attr(feature = "unstable", feature(array_methods))]

mod first_ok;
mod join;
mod merge;
mod race;
mod try_join;
mod utils;

/// The futures concurrency prelude.
pub mod prelude {
    pub use super::future::FirstOk;
    pub use super::future::Join;
    pub use super::future::Race;
    pub use super::future::TryJoin;
    pub use super::stream::IntoStream as _;
    pub use super::stream::Merge;
}

/// Asynchronous basic functionality.
///
/// Please see the fundamental `async` and `await` keywords and the [async book]
/// for more information on asynchronous programming in Rust.
///
/// [async book]: https://rust-lang.github.io/async-book/
///
/// # Examples
///
/// ```
/// use futures_concurrency::prelude::*;
/// use futures_lite::future::block_on;
/// use std::future;
///
/// fn main() {
///     block_on(async {
///         // Await multiple similarly-typed futures.
///         let a = future::ready(1);
///         let b = future::ready(2);
///         let c = future::ready(3);
///         assert_eq!([a, b, c].join().await, [1, 2, 3]);
///    
///         // Await multiple differently-typed futures.
///         let a = future::ready(1u8);
///         let b = future::ready("hello");
///         let c = future::ready(3u16);
///         assert_eq!((a, b, c).join().await, (1, "hello", 3));
///
///         // It even works with vectors of futures, providing an alternative
///         // to futures-rs' `join_all`.
///         let a = future::ready(1);
///         let b = future::ready(2);
///         let c = future::ready(3);
///         assert_eq!(vec![a, b, c].join().await, vec![1, 2, 3]);
///     })
/// }
/// ```
///
/// # Base Futures Concurrency
///
/// Often it's desireable to await multiple futures as if it was a single
/// future. The `join` family of operations converts multiple futures into a
/// single future that returns all of their outputs. The `race` family of
/// operations converts multiple future into a single future that returns the
/// first output.
///
/// For operating on futures the following functions can be used:
///
/// | Name     | Return signature | When does it return?     |
/// | ---      | ---              | ---                      |
/// | `Join`   | `(T1, T2)`       | Wait for all to complete
/// | `Race`   | `T`              | Return on  value
///
/// ## Fallible Futures Concurrency
///
/// For operating on futures that return `Result` additional `try_` variants of
/// the functions mentioned before can be used. These functions are aware of `Result`,
/// and will behave slightly differently from their base variants.
///
/// In the case of `try_join`, if any of the futures returns `Err` all
/// futures are dropped and an error is returned. This is referred to as
/// "short-circuiting".
///
/// In the case of `race_ok`, instead of returning the  future that
/// completes it returns the first future that _successfully_ completes. This
/// means `race_ok` will keep going until any one of the futures returns
/// `Ok`, or _all_ futures have returned `Err`.
///
/// However sometimes it can be useful to use the base variants of the functions
/// even on futures that return `Result`. Here is an overview of operations that
/// work on `Result`, and their respective semantics:
///
/// | Name        | Return signature               | When does it return? |
/// | ---         | ---                            | ---                  |
/// | `Join`      | `(Result<T, E>, Result<T, E>)` | Wait for all to complete
/// | `TryJoin`   | `Result<(T1, T2), E>`          | Return on  `Err`, wait for all to complete
/// | `Race`      | `Result<T, E>`                 | Return on  value
/// | `RaceOk`    | `Result<T, E>`                 | Return on  `Ok`, reject on last Err
///
pub mod future {
    pub use crate::first_ok::FirstOk;
    pub use crate::join::Join;
    pub use crate::race::Race;
    pub use crate::try_join::TryJoin;
}

/// Composable asynchronous iteration.
///
/// # Examples
///
/// Merge multiple streams to handle values as soon as they're ready, without
/// ever dropping a single value:
///
/// ```
/// use futures_concurrency::prelude::*;
/// use futures_lite::future::block_on;
/// use futures_lite::{stream, StreamExt};
///
/// fn main() {
///     block_on(async {
///         let a = stream::once(1);
///         let b = stream::once(2);
///         let c = stream::once(3);
///         let mut s = (a, b, c).merge();
///
///         let mut counter = 0;
///         s.for_each(|n| counter += n).await;
///         assert_eq!(counter, 6);
///     })
/// }
/// ```
///
/// # Concurrency
///
/// For streams we expose a single concurrency method: `merge`. This allows
/// multiple streams to be merged into one, with items handled as soon as
/// they're ready. By their nature streams can be short-circuited on a per-item
/// basis, so we don't need to decide up front how we want to handle errors.
///
/// | Name        | Return signature               | When does it return? |
/// | ---         | ---                            | ---                  |
/// | `Merge`     | `T`                            | Each value as soon as it's ready.
pub mod stream {
    pub use crate::merge::Merge;
    pub use crate::utils::IntoStream;
}

/// Helper functions and types for fixed-length arrays.
pub mod array {
    pub use crate::first_ok::array::{AggregateError, FirstOk};
    pub use crate::join::array::Join;
    pub use crate::merge::array::Merge;
    pub use crate::race::array::Race;
    pub use crate::try_join::array::TryJoin;
}

/// A contiguous growable array type with heap-allocated contents, written
/// `Vec<T>`.
pub mod vec {
    pub use crate::first_ok::vec::{AggregateError, FirstOk};
    pub use crate::join::vec::Join;
    pub use crate::race::vec::Race;
    pub use crate::try_join::vec::TryJoin;
}
