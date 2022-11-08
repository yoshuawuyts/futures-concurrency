use core::fmt;
use core::ops::{Deref, DerefMut};

/// A collection of errors.
#[repr(transparent)]
pub struct AggregateError<E, const N: usize> {
    inner: [E; N],
}

impl<E, const N: usize> AggregateError<E, N> {
    pub(super) fn new(inner: [E; N]) -> Self {
        Self { inner }
    }
}

impl<E: fmt::Debug, const N: usize> fmt::Debug for AggregateError<E, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for err in self.inner.as_ref() {
            list.entry(err);
        }
        list.finish()
    }
}

impl<E: fmt::Debug, const N: usize> fmt::Display for AggregateError<E, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl<E, const N: usize> Deref for AggregateError<E, N> {
    type Target = [E; N];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<E, const N: usize> DerefMut for AggregateError<E, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<E: fmt::Debug, const N: usize> std::error::Error for AggregateError<E, N> {}
