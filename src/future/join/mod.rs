use core::future::Future;

pub(crate) mod array;
pub(crate) mod tuple;
pub(crate) mod vec;

/// Wait for all futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the futures
/// in the same container type they we're created once all complete.
pub trait Join {
    /// The resulting output type.
    type Output;

    /// The [`Future`] implementation returned by this method.
    type Future: Future<Output = Self::Output>;

    /// Waits for multiple futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the futures
    /// in the same container type they we're created once all complete.
    ///
    /// # Examples
    /// Awaiting multiple futures of the same type can be done using either a vector
    /// or an array.
    /// ```rust
    /// #  futures::executor::block_on(async {
    /// use futures_concurrency::prelude::*;
    ///
    /// // these can be whatever kind of `Future` you have,
    /// // as long as it all have the exactly same type
    /// let fut1 = core::future::ready(1);
    /// let fut2 = core::future::ready(2);
    /// let fut3 = core::future::ready(3);
    ///
    /// let outputs = [fut1, fut2, fut3].join().await;
    /// assert_eq!(outputs, [1, 2, 3]);
    /// # })
    /// ```
    ///
    /// But in real world, you might want to wait multiple futures of
    /// different types - let's say you're awaiting on custom `async {}` blocks,
    /// or that your futures have different `Output` types - for that, you can
    /// `join` on tuples.
    /// ```rust
    /// #  futures::executor::block_on(async {
    /// use futures_concurrency::prelude::*;
    ///
    /// async fn some_async_fn() -> usize { 3 }
    ///
    /// // these can be whatever kind of `Future` you have,
    /// // even those that have totally different concrete types.
    /// let fut1 = core::future::ready(1);
    /// let fut2 = async { 2 };
    /// let fut3 = some_async_fn();
    /// //                       ^ NOTE: no `.await` here!
    ///
    /// let outputs = (fut1, fut2, fut3).join().await;
    /// assert_eq!(outputs, (1, 2, 3));
    /// # })
    /// ```
    ///
    /// <br><br>
    /// This function returns a new future which polls all futures concurrently.
    fn join(self) -> Self::Future;
}
