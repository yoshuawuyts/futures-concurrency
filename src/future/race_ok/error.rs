/// A collection of errors returned when [super::RaceOk] fails.
///
/// Example:
/// ```
/// # use std::future;
/// # use std::io::{Error, ErrorKind};
/// # use futures_concurrency::errors::AggregateError;
/// # use futures_concurrency::future::RaceOk;
/// # futures_lite::future::block_on(async {
/// let err = Error::new(ErrorKind::Other, "oh no");
/// let res: Result<&str, AggregateError<[Error; 2]>> = [future::ready(Ok("hello")), future::ready(Err(err))]
///     .race_ok()
///     .await;
/// assert_eq!(res.unwrap(), "hello");
/// # });
/// ```
#[derive(Debug)]
pub struct AggregateError<E> {
    /// The errors. Can be a Vec, and Array, or a Tuple.
    pub errors: E,
}
