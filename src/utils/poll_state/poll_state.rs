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
    /// Returns `true` if the metadata is [`Pending`][Self::Pending].
    #[must_use]
    #[inline]
    pub(crate) fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns `true` if the poll state is [`Ready`][Self::Ready].
    #[must_use]
    #[inline]
    pub(crate) fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Sets the poll state to [`Ready`][Self::Ready].
    #[inline]
    pub(crate) fn set_ready(&mut self) {
        *self = PollState::Ready;
    }

    /// Returns `true` if the poll state is [`Consumed`][Self::Consumed].
    #[must_use]
    #[inline]
    pub(crate) fn is_consumed(&self) -> bool {
        matches!(self, Self::Consumed)
    }

    /// Sets the poll state to [`Consumed`][Self::Consumed].
    #[inline]
    pub(crate) fn set_consumed(&mut self) {
        *self = PollState::Consumed;
    }
}
