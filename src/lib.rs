//! Concurrency extensions for Future

// #![deny(missing_debug_implementations, nonstandard_style)]
// #![warn(missing_docs, unreachable_pub)]

mod maybe_done;

use core::future::Future;

pub(crate) use maybe_done::MaybeDone;

pub trait Join<T, const N: usize>
where
    T: Future,
{
    type Input;
    type Output: Future;

    fn join(slice: Self::Input) -> Self::Output;
}

pub mod array {
    use super::{Join as JoinTrait, MaybeDone};

    use core::fmt;
    use core::future::Future;
    use core::mem;
    use core::pin::Pin;
    use core::task::{Context, Poll};
    use std::boxed::Box;
    use std::vec::Vec;

    impl<T, const N: usize> JoinTrait<T, N> for [T; N]
    where
        T: Future,
    {
        type Input = [T; N];
        type Output = JoinAll<T>;

        fn join(slice: Self::Input) -> Self::Output {
            JoinAll {
                elems: Box::pin(slice.map(MaybeDone::new)),
            }
        }
    }

    fn iter_pin_mut<T>(slice: Pin<&mut [T]>) -> impl Iterator<Item = Pin<&mut T>> {
        // Safety: `std` _could_ make this unsound if it were to decide Pin's
        // invariants aren't required to transmit through slices. Otherwise this has
        // the same safety as a normal field pin projection.
        unsafe { slice.get_unchecked_mut() }
            .iter_mut()
            .map(|t| unsafe { Pin::new_unchecked(t) })
    }

    /// Future for the [`join_all`] function.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct JoinAll<F>
    where
        F: Future,
    {
        elems: Pin<Box<[MaybeDone<F>]>>,
    }

    impl<F> fmt::Debug for JoinAll<F>
    where
        F: Future + fmt::Debug,
        F::Output: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("JoinAll")
                .field("elems", &self.elems)
                .finish()
        }
    }

    impl<F> Future for JoinAll<F>
    where
        F: Future,
    {
        type Output = Vec<F::Output>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut all_done = true;

            for elem in iter_pin_mut(self.elems.as_mut()) {
                if elem.poll(cx).is_pending() {
                    all_done = false;
                }
            }

            if all_done {
                let mut elems = mem::replace(&mut self.elems, Box::pin([]));
                let result = iter_pin_mut(elems.as_mut())
                    .map(|e| e.take().unwrap())
                    .collect();
                Poll::Ready(result)
            } else {
                Poll::Pending
            }
        }
    }
}

pub mod vec {
    use super::{Join as JoinTrait, MaybeDone};

    use core::fmt;
    use core::future::Future;
    use core::mem;
    use core::pin::Pin;
    use core::task::{Context, Poll};
    use std::boxed::Box;
    use std::vec::Vec;

    impl<T, const N: usize> JoinTrait<T, N> for Vec<T>
    where
        T: Future,
    {
        type Input = Vec<T>;
        type Output = JoinAll<T>;

        fn join(input: Vec<T>) -> Self::Output {
            let elems: Box<[_]> = input.into_iter().map(MaybeDone::new).collect();
            JoinAll {
                elems: elems.into(),
            }
        }
    }

    fn iter_pin_mut<T>(slice: Pin<&mut [T]>) -> impl Iterator<Item = Pin<&mut T>> {
        // Safety: `std` _could_ make this unsound if it were to decide Pin's
        // invariants aren't required to transmit through slices. Otherwise this has
        // the same safety as a normal field pin projection.
        unsafe { slice.get_unchecked_mut() }
            .iter_mut()
            .map(|t| unsafe { Pin::new_unchecked(t) })
    }

    /// Future for the [`join_all`] function.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct JoinAll<F>
    where
        F: Future,
    {
        elems: Pin<Box<[MaybeDone<F>]>>,
    }

    impl<F> fmt::Debug for JoinAll<F>
    where
        F: Future + fmt::Debug,
        F::Output: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("JoinAll")
                .field("elems", &self.elems)
                .finish()
        }
    }

    impl<F> Future for JoinAll<F>
    where
        F: Future,
    {
        type Output = Vec<F::Output>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut all_done = true;

            for elem in iter_pin_mut(self.elems.as_mut()) {
                if elem.poll(cx).is_pending() {
                    all_done = false;
                }
            }

            if all_done {
                let mut elems = mem::replace(&mut self.elems, Box::pin([]));
                let result = iter_pin_mut(elems.as_mut())
                    .map(|e| e.take().unwrap())
                    .collect();
                Poll::Ready(result)
            } else {
                Poll::Pending
            }
        }
    }
}
