pub(crate) mod array;
pub(crate) mod tuple;
pub(crate) mod vec;

/// Wait for all futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the futures
/// once both complete.
#[async_trait::async_trait(?Send)]
pub trait Join {
    /// The resulting output type.
    type Output;

    /// Waits for multiple futures to complete.
    ///
    /// Awaits multiple futures simultaneously, returning the output of the
    /// futures once both complete.
    ///
    /// This function returns a new future which polls both futures
    /// concurrently.
    async fn join(self) -> Self::Output;
}
