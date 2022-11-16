#![allow(unused)]

//! A type that wraps a future to keep track of its completion status.
//!
//! This implementation was taken from the original `macro_rules` `join/try_join`
//! macros in the `futures-preview` crate.

mod array;
mod fuse;
mod maybe_done;
mod pin;
mod poll_state;
mod rng;
mod tuple;
mod wakers;

pub(crate) use array::array_assume_init;
pub(crate) use fuse::Fuse;
pub(crate) use maybe_done::MaybeDone;
pub(crate) use pin::{get_pin_mut, get_pin_mut_from_vec, iter_pin_mut, iter_pin_mut_vec};
pub(crate) use poll_state::{PollState, PollStates};
pub(crate) use rng::RandomGenerator;
pub(crate) use tuple::{gen_conditions, permutations, tuple_len};
pub(crate) use wakers::{InlineWaker, Readiness, WakerList};

#[cfg(test)]
mod dummy_waker;

#[cfg(test)]
pub(crate) use dummy_waker::DummyWaker;
