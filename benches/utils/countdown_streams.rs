use futures_core::Stream;
use pin_project::pin_project;

use std::cell::{Cell, RefCell};
use std::collections::BinaryHeap;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use super::{shuffle, PrioritizedWaker, State};

pub fn streams_vec(len: usize) -> Vec<CountdownStream> {
    let wakers = Rc::new(RefCell::new(BinaryHeap::new()));
    let completed = Rc::new(Cell::new(0));
    let mut streams: Vec<_> = (0..len)
        .map(|n| CountdownStream::new(n, len, wakers.clone(), completed.clone()))
        .collect();
    shuffle(&mut streams);
    streams
}

pub fn streams_array<const N: usize>() -> [CountdownStream; N] {
    let wakers = Rc::new(RefCell::new(BinaryHeap::new()));
    let completed = Rc::new(Cell::new(0));
    let mut streams =
        std::array::from_fn(|n| CountdownStream::new(n, N, wakers.clone(), completed.clone()));
    shuffle(&mut streams);
    streams
}

pub fn streams_tuple() -> (
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
) {
    let [f0, f1, f2, f3, f4, f5, f6, f7, f8, f9] = streams_array::<10>();
    (f0, f1, f2, f3, f4, f5, f6, f7, f8, f9)
}

/// A stream which will _eventually_ be ready, but needs to be polled N times before it is.
#[pin_project]
pub struct CountdownStream {
    state: State,
    wakers: Rc<RefCell<BinaryHeap<PrioritizedWaker>>>,
    index: usize,
    max_count: usize,
    completed_count: Rc<Cell<usize>>,
}

impl CountdownStream {
    pub fn new(
        index: usize,
        max_count: usize,
        wakers: Rc<RefCell<BinaryHeap<PrioritizedWaker>>>,
        completed_count: Rc<Cell<usize>>,
    ) -> Self {
        Self {
            state: State::Init,
            wakers,
            max_count,
            index,
            completed_count,
        }
    }
}
impl Stream for CountdownStream {
    type Item = ();

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        // If we are the last stream to be polled, skip strait to the Polled state.
        if this.wakers.borrow().len() + 1 == *this.max_count {
            *this.state = State::Polled;
        }

        match this.state {
            State::Init => {
                // Push our waker onto the stack so we get woken again someday.
                this.wakers
                    .borrow_mut()
                    .push(PrioritizedWaker(*this.index, cx.waker().clone()));
                *this.state = State::Polled;
                Poll::Pending
            }
            State::Polled => {
                // Wake up the next one
                let _ = this
                    .wakers
                    .borrow_mut()
                    .pop()
                    .map(|PrioritizedWaker(_, waker)| waker.wake());

                if this.completed_count.get() == *this.index {
                    *this.state = State::Done;
                    this.completed_count.set(this.completed_count.get() + 1);
                    Poll::Ready(Some(()))
                } else {
                    // We're not done yet, so schedule another wakeup
                    this.wakers
                        .borrow_mut()
                        .push(PrioritizedWaker(*this.index, cx.waker().clone()));
                    Poll::Pending
                }
            }
            State::Done => Poll::Ready(None),
        }
    }
}
