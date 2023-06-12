//! Utilities to implement the different futures of this crate.

mod array;
mod futures;
mod indexer;
mod output;
mod pin;
mod poll_state;
mod tuple;
mod wakers;

pub(crate) use self::futures::{FutureArray, FutureVec};
pub(crate) use array::array_assume_init;
pub(crate) use indexer::Indexer;
pub(crate) use output::{OutputArray, OutputVec};
pub(crate) use pin::{get_pin_mut, get_pin_mut_from_vec, iter_pin_mut, iter_pin_mut_vec};
pub(crate) use poll_state::MaybeDone;
pub(crate) use poll_state::{PollArray, PollState, PollVec};
pub(crate) use tuple::{gen_conditions, tuple_len};
pub(crate) use wakers::{WakerArray, WakerVec};

#[cfg(test)]
pub(crate) use wakers::DummyWaker;

#[cfg(test)]
pub(crate) mod channel;
