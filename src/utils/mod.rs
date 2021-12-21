#![allow(unused)]

//! A type that wraps a future to keep track of its completion status.
//!
//! This implementation was taken from the original `macro_rules` `join/try_join`
//! macros in the `futures-preview` crate.

mod fuse;
mod maybe_done;
mod rng;

pub(crate) use fuse::Fuse;
pub(crate) use maybe_done::MaybeDone;
pub(crate) use rng::random;
