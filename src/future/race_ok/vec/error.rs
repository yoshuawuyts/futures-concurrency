use core::fmt;
use std::ops::Deref;
use std::ops::DerefMut;
use std::vec::Vec;

/// A collection of errors.
#[repr(transparent)]
pub struct AggregateError<E> {
    pub(crate) inner: Vec<E>,
}

impl<E> AggregateError<E> {
    pub(crate) fn new(inner: Vec<E>) -> Self {
        Self { inner }
    }
}

impl<E: fmt::Debug> fmt::Debug for AggregateError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for err in &self.inner {
            list.entry(err);
        }
        list.finish()
    }
}

impl<E: fmt::Debug> fmt::Display for AggregateError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<E> Deref for AggregateError<E> {
    type Target = Vec<E>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<E> DerefMut for AggregateError<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<E: fmt::Debug> std::error::Error for AggregateError<E> {}
