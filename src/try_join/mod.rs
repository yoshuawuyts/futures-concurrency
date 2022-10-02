pub(crate) mod vec;

/// Wait for all futures to complete successfully, or abort early on error.
///
/// In the case a future errors, all other futures will be cancelled. If
/// futures have been completed, their results will be discarded.
///
/// If you want to keep partial data in the case of failure, see the `merge`
/// operation.
#[async_trait::async_trait(?Send)]
pub trait TryJoin {
    /// The resulting output type.
    type Output;

    /// The resulting error type.
    type Error;

    /// Waits for multiple futures to complete, either returning when all
    /// futures complete successfully, or return early when any future completes
    /// with an error.
    async fn try_join(self) -> Result<Self::Output, Self::Error>;
}
