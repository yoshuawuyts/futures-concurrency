mod array;
#[cfg(test)]
mod dummy;
mod shared_slice_waker;
mod vec;

#[cfg(test)]
pub(crate) use dummy::DummyWaker;

pub(crate) use array::*;
pub(crate) use vec::*;
