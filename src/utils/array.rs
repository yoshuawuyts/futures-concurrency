use std::mem::{self, ManuallyDrop, MaybeUninit};

/// Extracts the values from an array of `MaybeUninit` containers.
///
/// # Safety
///
/// It is up to the caller to guarantee that all elements of the array are
/// in an initialized state.
///
/// Inlined version of: <https://doc.rust-lang.org/std/mem/union.MaybeUninit.html#method.array_assume_init>
pub(crate) unsafe fn array_assume_init<T, const N: usize>(array: [MaybeUninit<T>; N]) -> [T; N] {
    // SAFETY:
    // * The caller guarantees that all elements of the array are initialized
    // * `MaybeUninit<T>` and T are guaranteed to have the same layout
    // * `MaybeUninit` does not drop, so there are no double-frees
    // And thus the conversion is safe
    let ret = unsafe { (&array as *const _ as *const [T; N]).read() };

    // FIXME: required to avoid `~const Destruct` bound
    mem::forget(array);
    ret
}

/// Cast an array of `T` to an array of `MaybeUninit<T>`
pub(crate) fn array_to_manually_drop<T, const N: usize>(arr: [T; N]) -> [ManuallyDrop<T>; N] {
    // Implementation copied from: https://doc.rust-lang.org/src/core/mem/maybe_uninit.rs.html#1292
    let arr = MaybeUninit::new(arr);
    // SAFETY: T and MaybeUninit<T> have the same layout
    unsafe { mem::transmute_copy(&mem::ManuallyDrop::new(arr)) }
}
