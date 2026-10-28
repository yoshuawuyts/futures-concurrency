//! The regression futures hold mutable borrows of their own state across suspension.
//! Miri checks whether collection access invalidates the mutable borrows.

#![cfg(feature = "std")]

#[path = "suspended_borrows/auto_traits.rs"]
mod auto_traits;
#[path = "suspended_borrows/futures.rs"]
mod futures;
#[path = "suspended_borrows/groups.rs"]
mod groups;
#[path = "suspended_borrows/streams.rs"]
mod streams;
#[path = "suspended_borrows/support.rs"]
mod support;
