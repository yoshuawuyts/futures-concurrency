#![allow(unused)]

//! A type that wraps a future to keep track of its completion status.
//!
//! This implementation was taken from the original `macro_rules` `join/try_join`
//! macros in the `futures-preview` crate.

mod fuse;
mod iter_pin_mut;
mod maybe_done;
mod rng;

pub(crate) use fuse::Fuse;
pub(crate) use iter_pin_mut::{get_pin_mut, iter_pin_mut, pin_project_array};
pub(crate) use maybe_done::MaybeDone;
pub(crate) use rng::random;
