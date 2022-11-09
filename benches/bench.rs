use criterion::{black_box, criterion_group, criterion_main, Criterion};
use futures_concurrency::prelude::*;
use futures_core::Stream;
use futures_lite::future::block_on;
use futures_lite::prelude::*;
use pin_project::pin_project;

use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("merge 10", |b| b.iter(|| merge_test(black_box(10))));
    c.bench_function("merge 100", |b| b.iter(|| merge_test(black_box(100))));
    c.bench_function("merge 1000", |b| b.iter(|| merge_test(black_box(1000))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

pub(crate) fn merge_test(max: usize) {
    block_on(async {
        let wakers = Rc::new(RefCell::new(vec![]));
        let completed = Rc::new(RefCell::new(0));
        let futures: Vec<_> = (0..max)
            .map(|n| Countdown::new(n, max, wakers.clone(), completed.clone()))
            .collect();
        let mut s = futures.merge();

        let mut counter = 0;
        while s.next().await.is_some() {
            counter += 1;
        }
        assert_eq!(counter, max);
    })
}

#[derive(Clone, Copy)]
enum State {
    Init,
    Polled,
    Done,
}

/// A future which will _eventually_ be ready, but needs to be polled N times before it is.
#[pin_project]
struct Countdown {
    state: State,
    wakers: Rc<RefCell<Vec<Waker>>>,
    index: usize,
    max_count: usize,
    completed_count: Rc<RefCell<usize>>,
}

impl Countdown {
    fn new(
        index: usize,
        max_count: usize,
        wakers: Rc<RefCell<Vec<Waker>>>,
        completed_count: Rc<RefCell<usize>>,
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
impl Stream for Countdown {
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
                this.wakers.borrow_mut().push(cx.waker().clone());
                *this.state = State::Polled;
                Poll::Pending
            }
            State::Polled => {
                // Wake up the next one
                let _ = this.wakers.borrow_mut().pop().map(Waker::wake);

                if *this.completed_count.borrow() == *this.index {
                    *this.state = State::Done;
                    *this.completed_count.borrow_mut() += 1;
                    Poll::Ready(Some(()))
                } else {
                    // We're not done yet, so schedule another wakeup
                    this.wakers.borrow_mut().push(cx.waker().clone());
                    Poll::Pending
                }
            }
            State::Done => Poll::Ready(None),
        }
    }
}
