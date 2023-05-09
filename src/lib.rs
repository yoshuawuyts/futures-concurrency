//! Concurrency extensions for [`Future`][core::future::Future] and `Stream`
//! (also known as [`AsyncIterator`][core::async_iter::AsyncIterator]).
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
//! # Operations
//!
//! This library provides the following operations on arrays, vecs, and tuples:
//!
//! - [`future::Join`]: Wait for all futures to complete.
//! - [`future::TryJoin`]: Wait for all futures to complete successfully, or abort early on error.
//! - [`future::Race`]: Wait for the first future to complete.
//! - [`future::RaceOk`]: Wait for the first successful future to complete.
//! - [`stream::Chain`]: Takes multiple streams and creates a new stream over all in sequence.
//! - [`stream::Merge`]: Combines multiple streams into a single stream of all their outputs.
//! - [`stream::Zip`]: ‘Zips up’ multiple streams into a single stream of pairs.
//!
//! # Examples
//!
//! Concurrently await multiple heterogenous futures:
//! ```rust
//! use futures_concurrency::prelude::*;
//! use futures_lite::future::block_on;
//! use std::future;
//!
//! block_on(async {
//!     let a = future::ready(1u8);
//!     let b = future::ready("hello");
//!     let c = future::ready(3u16);
//!     assert_eq!((a, b, c).join().await, (1, "hello", 3));
//! })
//! ```
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

#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(missing_docs, unreachable_pub)]
#![allow(non_snake_case)]

mod utils;

/// The futures concurrency prelude.
pub mod prelude {
    pub use super::future::FutureExt as _;
    pub use super::stream::StreamExt as _;

    pub use super::future::Join as _;
    pub use super::future::Race as _;
    pub use super::future::RaceOk as _;
    pub use super::future::TryJoin as _;
    pub use super::stream::Chain as _;
    pub use super::stream::IntoStream as _;
    pub use super::stream::Merge as _;
    pub use super::stream::Zip as _;
}

pub mod future;
pub mod stream;

/// Helper functions and types for tuples.
pub mod tuple {
    pub use crate::future::join::tuple::{
        Join0, Join1, Join10, Join11, Join12, Join2, Join3, Join4, Join5, Join6, Join7, Join8,
        Join9,
    };
    pub use crate::future::race::tuple::{
        Race1, Race10, Race11, Race12, Race2, Race3, Race4, Race5, Race6, Race7, Race8, Race9,
    };
    pub use crate::future::race_ok::tuple::{
        RaceOk1, RaceOk10, RaceOk11, RaceOk12, RaceOk2, RaceOk3, RaceOk4, RaceOk5, RaceOk6,
        RaceOk7, RaceOk8, RaceOk9,
    };
    pub use crate::future::try_join::tuple::{
        TryJoin1, TryJoin10, TryJoin11, TryJoin12, TryJoin2, TryJoin3, TryJoin4, TryJoin5,
        TryJoin6, TryJoin7, TryJoin8, TryJoin9,
    };
    pub use crate::stream::chain::tuple::{
        Chain1, Chain10, Chain11, Chain12, Chain2, Chain3, Chain4, Chain5, Chain6, Chain7, Chain8,
        Chain9,
    };
    pub use crate::stream::merge::tuple::{
        Merge0, Merge1, Merge10, Merge11, Merge12, Merge2, Merge3, Merge4, Merge5, Merge6, Merge7,
        Merge8, Merge9,
    };
    pub use crate::stream::zip::tuple::{
        Zip1, Zip10, Zip11, Zip12, Zip2, Zip3, Zip4, Zip5, Zip6, Zip7, Zip8, Zip9,
    };
}

/// Helper functions and types for fixed-length arrays.
pub mod array {
    pub use crate::future::join::array::Join;
    pub use crate::future::race::array::Race;
    pub use crate::future::race_ok::array::{AggregateError, RaceOk};
    pub use crate::future::try_join::array::TryJoin;
    pub use crate::stream::chain::array::Chain;
    pub use crate::stream::merge::array::Merge;
    pub use crate::stream::zip::array::Zip;
}

/// Helper functions and types for contiguous growable array type with heap-allocated contents,
/// written `Vec<T>`.
pub mod vec {
    pub use crate::future::join::vec::Join;
    pub use crate::future::race::vec::Race;
    pub use crate::future::race_ok::vec::{AggregateError, RaceOk};
    pub use crate::future::try_join::vec::TryJoin;
    pub use crate::stream::chain::vec::Chain;
    pub use crate::stream::merge::vec::Merge;
    pub use crate::stream::zip::vec::Zip;
}
