//! Asynchronous values.

use crate::Join;

use async_trait::async_trait;
use core::future::Future;

/// Extend `Stream` with concurrency methods.
#[async_trait(?Send)]
pub trait FutureExt: Future {
    /// Join two differently-typed futures together.
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
    ///         let a = future::ready(1u8);
    ///         let b = future::ready("hello");
    ///         assert_eq!(a.join(b).await, (1, "hello"));
    ///     })
    /// }
    /// ```
    async fn join<F>(self, other: F) -> (Self::Output, F::Output)
    where
        Self: Sized,
        F: Future,
    {
        (self, other).join().await
    }
}

impl<S> FutureExt for S where S: Future {}
