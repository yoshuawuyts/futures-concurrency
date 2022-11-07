/// Enumerate the current poll state.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PollState {
    /// Actively polling the underlying future.
    Active,
    /// Data has been written to the output structure
    /// and the future should no longer be polled.
    Written,
    /// Data has been taken from the output structure,
    /// and we no longer need to reason about it.
    Taken,
}

impl PollState {
    /// Returns `true` if the poll state is [`Active`].
    ///
    /// [`Active`]: PollState::Active
    #[must_use]
    fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns `true` if the poll state is [`Done`].
    ///
    /// [`Done`]: PollState::Done
    #[must_use]
    fn is_done(&self) -> bool {
        matches!(self, Self::Written)
    }

    /// Returns `true` if the poll state is [`Taken`].
    ///
    /// [`Taken`]: PollState::Taken
    #[must_use]
    pub(crate) fn is_taken(&self) -> bool {
        matches!(self, Self::Taken)
    }
}

#[derive(Debug)]
pub(crate) struct Metadata {
    index: usize,
    poll_state: PollState,
}

impl Metadata {
    /// Create a new instance of `Metadata`, positioned at a certain index.
    pub(crate) fn new(index: usize) -> Self {
        Self {
            index,
            poll_state: PollState::Active,
        }
    }

    /// Get the index of the metadata.
    pub(crate) fn index(&self) -> usize {
        self.index
    }

    /// Get the current poll state.
    pub(crate) fn poll_state(&self) -> PollState {
        self.poll_state
    }

    /// Set the current poll state.
    pub(crate) fn set_poll_state(&mut self, poll_state: PollState) {
        self.poll_state = poll_state;
    }

    /// Set the current poll state to `Active`.
    pub(crate) fn set_active(&mut self) {
        self.poll_state = PollState::Active;
    }

    /// Set the current poll state to `Done`.
    pub(crate) fn set_done(&mut self) {
        self.poll_state = PollState::Written;
    }

    /// Set the current poll state to `Taken`.
    pub(crate) fn set_taken(&mut self) {
        self.poll_state = PollState::Taken;
    }

    /// Returns `true` if the poll state is [`Active`].
    ///
    /// [`Active`]: PollState::Active
    pub(crate) fn is_active(&self) -> bool {
        self.poll_state.is_active()
    }

    /// Returns `true` if the poll state is [`Done`].
    ///
    /// [`Done`]: PollState::Done
    pub(crate) fn is_done(&self) -> bool {
        self.poll_state.is_done()
    }

    /// Returns `true` if the poll state is [`Taken`].
    ///
    /// [`Taken`]: PollState::Taken
    pub(crate) fn is_taken(&self) -> bool {
        self.poll_state.is_taken()
    }
}
