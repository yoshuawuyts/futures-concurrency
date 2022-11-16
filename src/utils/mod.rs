#![allow(unused)]

//! A type that wraps a future to keep track of its completion status.
//!
//! This implementation was taken from the original `macro_rules` `join/try_join`
//! macros in the `futures-preview` crate.

mod array;
mod fuse;
mod pin;
mod poll_state;
mod rng;
mod tuple;
mod wakers;

pub(crate) use array::array_assume_init;
pub(crate) use fuse::Fuse;
pub(crate) use pin::{get_pin_mut, get_pin_mut_from_vec, iter_pin_mut, iter_pin_mut_vec};
pub(crate) use poll_state::MaybeDone;
pub(crate) use poll_state::{PollArray, PollState, PollVec};
pub(crate) use rng::RandomGenerator;
pub(crate) use tuple::{gen_conditions, permutations, tuple_len};
pub(crate) use wakers::{InlineWakerArray, WakerArray, WakerVec};

#[cfg(test)]
pub(crate) use wakers::DummyWaker;

#[cfg(test)]
pub(crate) mod channel;
