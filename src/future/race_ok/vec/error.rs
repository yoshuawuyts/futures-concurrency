use core::fmt;
use std::error::Error;
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

impl<E: Error> fmt::Debug for AggregateError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{self}:")?;

        for (i, err) in self.inner.iter().enumerate() {
            writeln!(f, "- Error {}: {err}", i + 1)?;
            let mut source = err.source();
            while let Some(err) = source {
                writeln!(f, "  ↳ Caused by: {err}")?;
                source = err.source();
            }
        }

        Ok(())
    }
}

impl<E: Error> fmt::Display for AggregateError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} errors occurred", self.inner.len())
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

impl<E: Error> Error for AggregateError<E> {}
