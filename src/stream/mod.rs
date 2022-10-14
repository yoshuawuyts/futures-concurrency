//! Composable asynchronous iteration.
//!
//! # Examples
//!
//! Merge multiple streams to handle values as soon as they're ready, without
//! ever dropping a single value:
//!
//! ```
//! use futures_concurrency::prelude::*;
//! use futures_lite::future::block_on;
//! use futures_lite::{stream, StreamExt};
//!
//! fn main() {
//!     block_on(async {
//!         let a = stream::once(1);
//!         let b = stream::once(2);
//!         let c = stream::once(3);
//!         let mut s = (a, b, c).merge();
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
//! For streams we expose a single concurrency method: `merge`. This allows
//! multiple streams to be merged into one, with items handled as soon as
//! they're ready. By their nature streams can be short-circuited on a per-item
//! basis, so we don't need to decide up front how we want to handle errors.
//!
//! | Name        | Return signature               | When does it return? |
//! | ---         | ---                            | ---                  |
//! | `Merge`     | `T`                            | Each value as soon as it's ready.
pub use crate::utils::IntoStream;
pub use merge::Merge;

mod merge;
