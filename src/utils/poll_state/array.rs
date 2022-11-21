use std::ops::{Deref, DerefMut};

use super::PollState;

pub(crate) struct PollArray<const N: usize> {
    state: [PollState; N],
}

impl<const N: usize> PollArray<N> {
    pub(crate) fn new() -> Self {
        Self {
            state: [PollState::default(); N],
        }
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
