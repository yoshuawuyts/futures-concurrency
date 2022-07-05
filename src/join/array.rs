use super::Join as JoinTrait;
use crate::utils::MaybeDone;

use core::fmt;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::sync::Arc;
use std::task::{Waker, Wake};

use pin_project::pin_project;

#[async_trait::async_trait(?Send)]
impl<T, const N: usize> JoinTrait for [T; N]
where
    T: Future,
{
    type Output = [T::Output; N];

    async fn join(self) -> Self::Output {
        let len = self.len();
        let mut i = 0;
        let elems = self.map(|f| {
            let ret = Elem::new(f, (i == len).then(|| i + 1));
            i += 1;
            ret
        });
        Join {
            elems,
            poll_next: if len > 0 { Some(0) } else { None },
        }
        .await
    }
}

/// A future + index of the next future to poll.
struct Elem<F: Future> {
    elem: MaybeDone<F>,
    poll_next: Option<usize>,
}

impl<F: Future> Elem<F> {
    fn new(fut: F, index: Option<usize>) -> Self {
        Self {
            elem: MaybeDone::new(fut),
            poll_next: index,
            done_count = 0,
        }
    }
}

impl<F> fmt::Debug for Elem<F>
where
    F: Future + fmt::Debug,
    F::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Elem").field("elem", &self.elem).finish()
    }
}

/// Waits for two similarly-typed futures to complete.
///
/// Awaits multiple futures simultaneously, returning the output of the
/// futures once both complete.
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
pub struct Join<F, const N: usize>
where
    F: Future,
{
    elems: [Elem<F>; N],
    poll_next: Option<usize>,
    done_count: usize,
}

/// When polling `Join`
///
/// 1. let t = self.elems[poll_next].poll(waker_for_i)
/// 2. self.poll_next = self.elems[poll_next].next
/// 3. if t is Pending, continue
/// 4. if t is complete, also do nothing...
///
/// This is in a loop until we traverse the whole list

/// Wakers
///
/// We need a waker per element
///
/// waker.wake(i):
/// 1. self.elems[i].next = self.poll_next
/// 2. self.poll_next = i
/// 3. call parent waker

impl<F, const N: usize> fmt::Debug for Join<F, N>
where
    F: Future + fmt::Debug,
    F::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Join").field("elems", &self.elems).finish()
    }
}

impl<F, const N: usize> Future for Join<F, N>
where
    F: Future,
{
    type Output = [F::Output; N];

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let unsafe_self = unsafe { self.get_unchecked_mut() };
        let this = self.project();

        let mut next = *this.poll_next;
        loop {
            match next {
                Some(i) => {
                    let elem = unsafe { Pin::new_unchecked(&mut this.elems[i].elem) };
                    *this.poll_next = this.elems[i].poll_next;
                    // FIXME: need to create a waker for this element
                    let waker = ElemWaker {
                        elem: i,
                        join: unsafe_self, // address of self
                        parent: cx.waker().clone(),
                    };
                    if elem.poll(&mut Context::from_waker(&Arc::new(waker).into())).is_ready() {
                        self.done_count += 1;
                    }

                    // We need to make sure this line comes after elem.poll, since elem.poll might
                    // wake itself before returning Pending
                    next = *this.poll_next;
                }
                None => break,
            }
        }

        if *this.done_count == N {
            use core::mem::MaybeUninit;

            // Create the result array based on the indices
            let mut out: [MaybeUninit<F::Output>; N] = {
                // inlined version of unstable `MaybeUninit::uninit_array()`
                // TODO: replace with `MaybeUninit::uninit_array()` when it becomes stable
                unsafe { MaybeUninit::<[MaybeUninit<_>; N]>::uninit().assume_init() }
            };

            // NOTE: this clippy attribute can be removed once we can `collect` into `[usize; K]`.
            #[allow(clippy::clippy::needless_range_loop)]
            for (i, el) in this.elems.iter_mut().enumerate() {
                let el = unsafe { Pin::new_unchecked(&mut el.elem) }.take().unwrap();
                out[i] = MaybeUninit::new(el);
            }
            let result = unsafe { out.as_ptr().cast::<[F::Output; N]>().read() };
            Poll::Ready(result)
        } else {
            Poll::Pending
        }
    }
}

struct ElemWaker<F: Future, const N: usize> {
    elem: usize,
    join: *mut Join<F, N>,
    parent: Waker,
}

impl<F: Future, const N: usize> Wake for ElemWaker<F, N> {
    fn wake(self: Arc<Self>) {
        let next = self.join.poll_next;
        let me = &mut self.join.elems[self.elem];
        self.join.poll_next = Some(self.elem);
        match next {
            Some(i) => {
                me.poll_next = Some(i)
            },
            None => {
                self.parent.wake()
            }
        }
    }
}
