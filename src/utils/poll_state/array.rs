use std::ops::{Deref, DerefMut};

use super::PollState;

pub(crate) struct PollArray<const N: usize> {
    state: [PollState; N],
}

impl<const N: usize> PollArray<N> {
    pub(crate) fn new(len: usize) -> Self {
        Self {
            state: [PollState::default(); N],
        }
    }

    /// Reset the `poll_state` back to its previous setting.
    pub(crate) fn reset(&mut self) {
        self.state.fill(PollState::default());
    }
}

impl<const N: usize> Deref for PollArray<N> {
    type Target = [PollState];

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<const N: usize> DerefMut for PollArray<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}
