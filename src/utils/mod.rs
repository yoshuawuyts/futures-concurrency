#![allow(unused)]

//! A type that wraps a future to keep track of its completion status.
//!
//! This implementation was taken from the original `macro_rules` `join/try_join`
//! macros in the `futures-preview` crate.

mod fuse;
mod maybe_done;
mod pin;
mod rng;

pub(crate) use fuse::Fuse;
pub(crate) use maybe_done::MaybeDone;
pub(crate) use pin::{get_pin_mut, get_pin_mut_from_vec, iter_pin_mut};
pub(crate) use rng::random;

#[cfg(feature = "unstable")]
pub(crate) use pin::pin_project_array;
