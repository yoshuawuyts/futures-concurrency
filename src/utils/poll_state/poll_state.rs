/// Enumerate the current poll state.
#[derive(Debug, Clone, Copy, Default)]
#[repr(u8)]
pub(crate) enum PollState {
    /// Polling the underlying future or stream.
    #[default]
    Pending,
    /// Data has been written to the output structure, and is now ready to be
    /// read.
    Ready,
    /// The underlying future or stream has finished yielding data and all data
    /// has been read. We can now stop reasoning about it.
    Consumed,
}

impl PollState {
    /// Returns `true` if the metadata is [`Pending`].
    ///
    /// [`Pending`]: Metadata::Pending
    #[must_use]
    #[inline]
    pub(crate) fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns `true` if the poll state is [`Ready`].
    ///
    /// [`Done`]: PollState::Done
    #[must_use]
    #[inline]
    pub(crate) fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Returns `true` if the poll state is [`Consumed`].
    ///
    /// [`Consumed`]: PollState::Consumed
    #[must_use]
    #[inline]
    pub(crate) fn is_consumed(&self) -> bool {
        matches!(self, Self::Consumed)
    }
}
