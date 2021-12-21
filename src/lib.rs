//! Concurrency extensions for `Future`.
//!
//! # Examples
//!
//! ```
//! use futures_concurrency::prelude::*;
//! use futures_lite::future::block_on;
//! use std::future;
//!
//! fn main() {
//!     block_on(async {
//!         // Await multiple similarly-typed futures.
//!         let a = future::ready(1u8);
//!         let b = future::ready(2u8);
//!         let c = future::ready(3u8);
//!         assert_eq!([a, b, c].join().await, [1, 2, 3]);
//!    
//!         // Await multiple differently-typed futures.
//!         let a = future::ready(1u8);
//!         let b = future::ready("hello");
//!         let c = future::ready(3u16);
//!         assert_eq!((a, b, c).join().await, (1, "hello", 3));
//!
//!         // It even works with vectors of futures, providing an alternative
//!         // to futures-rs' `join_all`.
//!         let a = future::ready(1u8);
//!         let b = future::ready(2u8);
//!         let c = future::ready(3u8);
//!         assert_eq!(vec![a, b, c].join().await, vec![1, 2, 3]);
//!     })
//! }
//! ```
//!
//! # Progress
//!
//! The following traits have been implemented.
//!
//! - [x] `Join`
//! - [x] `Merge`
//! - [ ] `TryJoin`
//! - [ ] `Race`
//! - [ ] `TryRace`
//!
//! # Base Futures Concurrency
//!
//! Often it's desireable to await multiple futures as if it was a single
//! future. The `join` family of operations converts multiple futures into a
//! single future that returns all of their outputs. The `race` family of
//! operations converts multiple future into a single future that returns the
//! first output.
//!
//! For operating on futures the following functions can be used:
//!
//! | Name     | Return signature | When does it return?     |
//! | ---      | ---              | ---                      |
//! | `Join`   | `(T1, T2)`       | Wait for all to complete
//! | `Race`   | `T`              | Return on first value
//!
//! ## Fallible Futures Concurrency
//!
//! For operating on futures that return `Result` additional `try_` variants of
//! the functions mentioned before can be used. These functions are aware of `Result`,
//! and will behave slightly differently from their base variants.
//!
//! In the case of `try_join`, if any of the futures returns `Err` all
//! futures are dropped and an error is returned. This is referred to as
//! "short-circuiting".
//!
//! In the case of `try_race`, instead of returning the first future that
//! completes it returns the first future that _successfully_ completes. This
//! means `try_race` will keep going until any one of the futures returns
//! `Ok`, or _all_ futures have returned `Err`.
//!
//! However sometimes it can be useful to use the base variants of the functions
//! even on futures that return `Result`. Here is an overview of operations that
//! work on `Result`, and their respective semantics:
//!
//! | Name        | Return signature               | When does it return? |
//! | ---         | ---                            | ---                  |
//! | `Join`      | `(Result<T, E>, Result<T, E>)` | Wait for all to complete
//! | `TryJoin`   | `Result<(T1, T2), E>`          | Return on first `Err`, wait for all to complete
//! | `Race`      | `Result<T, E>`                 | Return on first value
//! | `Try_race`  | `Result<T, E>`                 | Return on first `Ok`, reject on last Err

#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(missing_docs, unreachable_pub)]
#![allow(non_snake_case)]
#![feature(maybe_uninit_uninit_array)]
#![feature(array_methods)]
#![feature(array_from_fn)]
#![feature(pin_deref_mut)]

mod join;
mod merge;

pub mod future;
pub mod stream;

pub(crate) mod utils;

pub use join::Join;
pub use merge::Merge;

/// The futures concurrency prelude.
pub mod prelude {
    pub use super::future::FutureExt as _;
    pub use super::stream::IntoStream as _;
    pub use super::stream::StreamExt as _;
    pub use super::Join;
    pub use super::Merge;
}

/// Implementations for the tuple type.
///
/// No types are publicly exposed here currently. All tuple return types are
/// currently private, but you can call `join`, `merge`, and other methods on
/// tuples without any issue.
pub mod tuple {
    // pub use crate::join::tuple::*;
}

/// Implementations for the array type.
pub mod array {
    pub use crate::join::array::Join;
    pub use crate::merge::array::Merge;
}

/// Implementations for the vec type.
pub mod vec {
    pub use crate::join::vec::*;
}
