//! Utilities to implement the different futures of this crate.

mod array;
mod array_dequeue;
mod pin;
mod poll_state;
mod wakers;

pub(crate) use array::array_assume_init;
pub(crate) use array_dequeue::ArrayDequeue;
pub(crate) use pin::{get_pin_mut, get_pin_mut_from_vec, iter_pin_mut, iter_pin_mut_vec};
pub(crate) use poll_state::PollState;
pub(crate) use wakers::{dummy_waker, WakerArray, WakerVec};

#[cfg(test)]
pub(crate) mod channel;
