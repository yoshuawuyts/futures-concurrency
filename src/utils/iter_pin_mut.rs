use core::{array::IntoIter, pin::Pin};
use std::{iter::Map, slice::SliceIndex};

pub(crate) fn iter_pin_mut<T>(slice: Pin<&mut [T]>) -> impl Iterator<Item = Pin<&mut T>> {
    // SAFETY: `std` _could_ make this unsound if it were to decide Pin's
    // invariants aren't required to transmit through slices. Otherwise this has
    // the same safety as a normal field pin projection.
    unsafe { slice.get_unchecked_mut() }
        .iter_mut()
        .map(|t| unsafe { Pin::new_unchecked(t) })
}

pub(crate) fn pin_project_array<T, const N: usize>(slice: Pin<&mut [T; N]>) -> [Pin<&mut T>; N] {
    // SAFETY: `std` _could_ make this unsound if it were to decide Pin's
    // invariants aren't required to transmit through arrays. Otherwise this has
    // the same safety as a normal field pin projection.
    unsafe { slice.get_unchecked_mut() }
        .each_mut()
        .map(|t| unsafe { Pin::new_unchecked(t) })
}

/// Returns a pinned mutable reference to an element or subslice depending on the
/// type of index (see [`get`]) or `None` if the index is out of bounds.
///
/// [`get`]: #method.get
///
/// # Examples
///
/// ```
/// #![feature(slice_get_pin)]
/// use std::pin::Pin;
///
/// let v = vec![0, 1, 2].into_boxed_slice();
/// let mut pinned = Pin::from(v);
///
/// if let Some(mut elem) = pinned.as_mut().get_pin_mut(1) {
///     elem.set(10);
/// }
/// assert_eq!(&*pinned, &[0, 10, 2]);
/// ```
#[inline]
pub(crate) fn get_pin_mut<T, I>(slice: Pin<&mut [T]>, index: I) -> Option<Pin<&mut I::Output>>
where
    I: SliceIndex<[T]>,
{
    // SAFETY: `get_unchecked_mut` is never used to move the slice inside `self` (`SliceIndex`
    // is sealed and all `SliceIndex::get_mut` implementations never move elements).
    // `x` is guaranteed to be pinned because it comes from `self` which is pinned.
    unsafe {
        slice
            .get_unchecked_mut()
            .get_mut(index)
            .map(|x| Pin::new_unchecked(x))
    }
}
