use core::future::Future;

pub(crate) mod array;
pub(crate) mod tuple;
pub(crate) mod vec;

/// Wait for the first future to complete.
///
/// Awaits multiple futures simultaneously, returning as soon as one completes, even if it
/// completes with an error. All remaining futures are cancelled.
pub trait Race {
    /// The resulting output type.
    type Output;

    /// The [`Future`] implementation returned by this method.
    type Future: Future<Output = Self::Output>;

    /// Wait for the first future to complete.
    ///
    /// Awaits multiple futures simultaneously, returning as soon as one completes, even if it
    /// completes with an error. All remaining futures are cancelled.
    ///
    /// All futures must resolve to the same type.
    ///
    /// Use [`race_ok`] if you only care about the first successful future to complete.
    ///
    /// # Examples
    ///
    /// Await multiple futures until the first resolve.
    ///
    /// ```rust
    /// # futures::executor::block_on(async {
    /// use futures_concurrency::prelude::*;
    ///
    /// async fn fast_and_err(id: u8) -> Result<u8, u8> {
    ///     futures_lite::future::yield_now().await;
    ///     Err(id)
    /// }
    /// async fn slow_and_ok(id: u8) -> Result<u8, u8> {
    ///     futures_lite::future::yield_now().await;
    ///     Ok(id)
    /// }
    ///
    /// let id = (slow_and_ok(0), fast_and_err(1)).race().await;
    ///
    /// assert_eq!(id, Err(1));
    /// # });
    /// ```
    ///
    /// ## Translating from `select!`
    ///
    /// The following example is taken from the [`futures::select!`] documentation:
    ///
    /// ```rust
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::select;
    /// let mut a = future::ready(4);
    /// let mut b = future::pending::<()>();
    ///
    /// let res = select! {
    ///     a_res = a => a_res + 1,
    ///     _ = b => 0,
    /// };
    /// assert_eq!(res, 5);
    /// # });
    /// ```
    ///
    /// and can be translated using `race` as:
    ///
    /// ```rust
    /// # futures::executor::block_on(async {
    /// use futures::future;
    /// use futures::select;
    /// use futures_concurrency::prelude::*;
    ///
    /// let mut a = async {
    ///     let a_res = future::ready(4).await;
    ///     a_res + 1
    /// };
    /// let mut b = async {
    ///     let _ = future::pending::<()>().await;
    ///     0
    /// };
    ///
    /// let res = (a, b).race().await;
    ///
    /// assert_eq!(res, 5);
    /// # });
    /// ```
    ///
    /// <br><br>
    /// This function returns a new future which polls all futures concurrently.
    ///
    /// [`race_ok`]: crate::future::RaceOk::race_ok
    /// [`futures::select!`]: https://docs.rs/futures/0.3.28/futures/macro.select.html#examples
    fn race(self) -> Self::Future;
}
