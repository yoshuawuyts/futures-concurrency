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

mod utils;

/// The futures concurrency prelude.
pub mod prelude {
    pub use super::future::FirstOk as _;
    pub use super::future::Merge as _;
    pub use super::future::Race as _;
    pub use super::future::TryMerge as _;
    pub use super::stream::Merge as _;
    pub use super::utils::IntoStream as _;
}

pub mod future;
pub mod stream;
