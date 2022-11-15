/// Enumerate the current poll state.
#[derive(Debug, Clone, Copy, Default)]
#[repr(u8)]
pub(crate) enum PollState {
    /// Polling the underlying future.
    #[default]
    Pending,
    /// Data has been written to the output structure
    /// and the future should no longer be polled.
    Done,
    /// Data has been consumed from the output structure,
    /// and we should no longer reason about it.
    Consumed,
}

impl PollState {
    /// Returns `true` if the metadata is [`Pending`].
    ///
    /// [`Pending`]: Metadata::Pending
    #[must_use]
    pub(crate) fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns `true` if the poll state is [`Done`].
    ///
    /// [`Done`]: PollState::Done
    #[must_use]
    pub(crate) fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }

    /// Returns `true` if the poll state is [`Consumed`].
    ///
    /// [`Consumed`]: PollState::Consumed
    #[must_use]
    pub(crate) fn is_consumed(&self) -> bool {
        matches!(self, Self::Consumed)
    }
}
