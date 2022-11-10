use criterion::{black_box, criterion_group, criterion_main, Criterion};
use futures_concurrency::prelude::*;
use futures_lite::future::block_on;
use futures_lite::prelude::*;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

fn vec_merge_bench(c: &mut Criterion) {
    c.bench_function("vec::merge 10", |b| b.iter(|| vec_merge(black_box(10))));
    c.bench_function("vec::merge 100", |b| b.iter(|| vec_merge(black_box(100))));
    c.bench_function("vec::merge 1000", |b| b.iter(|| vec_merge(black_box(1000))));
}

fn vec_join_bench(c: &mut Criterion) {
    c.bench_function("vec::join 10", |b| b.iter(|| vec_join(black_box(10))));
    c.bench_function("vec::join 100", |b| b.iter(|| vec_join(black_box(100))));
    c.bench_function("vec::join 1000", |b| b.iter(|| vec_join(black_box(1000))));
}

criterion_group!(merge_benches, vec_merge_bench);
criterion_group!(join_benches, vec_join_bench);
criterion_main!(merge_benches, join_benches);

pub(crate) fn vec_merge(max: usize) {
    block_on(async {
        let wakers = Rc::new(RefCell::new(VecDeque::new()));
        let completed = Rc::new(RefCell::new(0));
        let futures: Vec<_> = (0..max)
            .map(|n| utils::CountdownStream::new(n, max, wakers.clone(), completed.clone()))
            .collect();
        let mut s = futures.merge();

        let mut counter = 0;
        while s.next().await.is_some() {
            counter += 1;
        }
        assert_eq!(counter, max);
    })
}

pub(crate) fn vec_join(max: usize) {
    block_on(async {
        let wakers = Rc::new(RefCell::new(VecDeque::new()));
        let completed = Rc::new(RefCell::new(0));
        let futures: Vec<_> = (0..max)
            .map(|n| utils::CountdownFuture::new(n, max, wakers.clone(), completed.clone()))
            .collect();
        let outputs = futures_concurrency::future::Join::join(futures).await;
        assert_eq!(outputs.len(), max);
    })
}

mod utils {
    use futures_core::Stream;
    use futures_lite::prelude::*;
    use pin_project::pin_project;

    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};

    #[derive(Clone, Copy)]
    enum State {
        Init,
        Polled,
        Done,
    }

    /// A stream which will _eventually_ be ready, but needs to be polled N times before it is.
    #[pin_project]
    pub(crate) struct CountdownStream {
        state: State,
        wakers: Rc<RefCell<VecDeque<Waker>>>,
        index: usize,
        max_count: usize,
        completed_count: Rc<RefCell<usize>>,
    }

    impl CountdownStream {
        pub(crate) fn new(
            index: usize,
            max_count: usize,
            wakers: Rc<RefCell<VecDeque<Waker>>>,
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
                    this.wakers.borrow_mut().push_back(cx.waker().clone());
                    *this.state = State::Polled;
                    Poll::Pending
                }
                State::Polled => {
                    // Wake up the next one
                    let _ = this.wakers.borrow_mut().pop_front().map(Waker::wake);

                    if *this.completed_count.borrow() == *this.index {
                        *this.state = State::Done;
                        *this.completed_count.borrow_mut() += 1;
                        Poll::Ready(Some(()))
                    } else {
                        // We're not done yet, so schedule another wakeup
                        this.wakers.borrow_mut().push_back(cx.waker().clone());
                        Poll::Pending
                    }
                }
                State::Done => Poll::Ready(None),
            }
        }
    }

    /// A future which will _eventually_ be ready, but needs to be polled N times before it is.
    #[pin_project]
    pub(crate) struct CountdownFuture {
        state: State,
        wakers: Rc<RefCell<VecDeque<Waker>>>,
        index: usize,
        max_count: usize,
        completed_count: Rc<RefCell<usize>>,
    }

    impl CountdownFuture {
        pub(crate) fn new(
            index: usize,
            max_count: usize,
            wakers: Rc<RefCell<VecDeque<Waker>>>,
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
    impl Future for CountdownFuture {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.project();

            // If we are the last stream to be polled, skip strait to the Polled state.
            if this.wakers.borrow().len() + 1 == *this.max_count {
                *this.state = State::Polled;
            }

            match this.state {
                State::Init => {
                    // Push our waker onto the stack so we get woken again someday.
                    this.wakers.borrow_mut().push_back(cx.waker().clone());
                    *this.state = State::Polled;
                    Poll::Pending
                }
                State::Polled => {
                    // Wake up the next one
                    let _ = this.wakers.borrow_mut().pop_front().map(Waker::wake);

                    if *this.completed_count.borrow() == *this.index {
                        *this.state = State::Done;
                        *this.completed_count.borrow_mut() += 1;
                        Poll::Ready(())
                    } else {
                        // We're not done yet, so schedule another wakeup
                        this.wakers.borrow_mut().push_back(cx.waker().clone());
                        Poll::Pending
                    }
                }
                State::Done => Poll::Ready(()),
            }
        }
    }
}
