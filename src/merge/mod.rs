use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

pub(crate) mod array;
