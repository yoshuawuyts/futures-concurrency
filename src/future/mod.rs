//! Asynchronous basic functionality.
//!
//! Please see the fundamental `async` and `await` keywords and the [async book]
//! for more information on asynchronous programming in Rust.
//!
//! [async book]: https://rust-lang.github.io/async-book/
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
//!         let a = future::ready(1);
//!         let b = future::ready(2);
//!         let c = future::ready(3);
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
//!         let a = future::ready(1);
//!         let b = future::ready(2);
//!         let c = future::ready(3);
//!         assert_eq!(vec![a, b, c].join().await, vec![1, 2, 3]);
//!     })
//! }
//! ```
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
//! | `Race`   | `T`              | Return on  value
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
//! In the case of `race_ok`, instead of returning the  future that
//! completes it returns the first future that _successfully_ completes. This
//! means `race_ok` will keep going until any one of the futures returns
//! `Ok`, or _all_ futures have returned `Err`.
//!
//! However sometimes it can be useful to use the base variants of the functions
//! even on futures that return `Result`. Here is an overview of operations that
//! work on `Result`, and their respective semantics:
//!
//! | Name        | Return signature               | When does it return? |
//! | ---         | ---                            | ---                  |
//! | `Join`      | `(Result<T, E>, Result<T, E>)` | Wait for all to complete
//! | `TryJoin`   | `Result<(T1, T2), E>`          | Return on  `Err`, wait for all to complete
//! | `Race`      | `Result<T, E>`                 | Return on  value
//! | `RaceOk`    | `Result<T, E>`                 | Return on  `Ok`, reject on last Err
//!
pub use first_ok::FirstOk;
pub use join::Join;
pub use race::Race;
pub use try_join::TryJoin;

mod first_ok;
mod join;
mod race;
mod try_join;
