// pub(crate) mod array;
pub(crate) mod vec;

/// Wait for the first successful future to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the first
/// future which completes. If no future completes successfully, returns an
/// aggregate error of all failed futures.
#[async_trait::async_trait(?Send)]
pub trait FirstOk {
    /// The resulting output type.
    type Output;

    /// The resulting error type.
    type Error;

    /// Waits for the first successful future to complete.
    async fn first_ok(self) -> Result<Self::Output, Self::Error>;
}
