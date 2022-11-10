criterion::criterion_main!(merge::merge_benches, join::join_benches, race::race_benches);

mod merge {
    use criterion::{black_box, criterion_group, Criterion};
    use futures_concurrency::prelude::*;
    use futures_lite::future::block_on;
    use futures_lite::prelude::*;

    use super::utils::{streams_array, streams_tuple, streams_vec};

    criterion_group!(
        merge_benches,
        vec_merge_bench,
        array_merge_bench,
        tuple_merge_bench
    );

    fn vec_merge_bench(c: &mut Criterion) {
        c.bench_function("vec::merge 10", |b| b.iter(|| vec_merge(black_box(10))));
        c.bench_function("vec::merge 100", |b| b.iter(|| vec_merge(black_box(100))));
        c.bench_function("vec::merge 1000", |b| b.iter(|| vec_merge(black_box(1000))));
    }

    fn array_merge_bench(c: &mut Criterion) {
        c.bench_function("array::merge 10", |b| b.iter(|| array_merge::<10>()));
        c.bench_function("array::merge 100", |b| b.iter(|| array_merge::<100>()));
        c.bench_function("array::merge 1000", |b| b.iter(|| array_merge::<1000>()));
    }

    fn tuple_merge_bench(c: &mut Criterion) {
        c.bench_function("tuple::merge 10", |b| b.iter(|| tuple_merge()));
    }

    pub(crate) fn vec_merge(max: usize) {
        block_on(async {
            let mut counter = 0;
            let streams = streams_vec(max);
            let mut s = streams.merge();
            while s.next().await.is_some() {
                counter += 1;
            }
            assert_eq!(counter, max);
        })
    }

    pub(crate) fn array_merge<const N: usize>() {
        block_on(async move {
            let mut counter = 0;
            let streams = streams_array::<N>();
            let mut s = streams.merge();
            while s.next().await.is_some() {
                counter += 1;
            }
            assert_eq!(counter, N);
        })
    }

    pub(crate) fn tuple_merge() {
        block_on(async move {
            let mut counter = 0;
            let streams = streams_tuple();
            let mut s = streams.merge();
            while s.next().await.is_some() {
                counter += 1;
            }
            assert_eq!(counter, 10);
        })
    }
}

mod join {
    use criterion::{black_box, criterion_group, Criterion};
    use futures_lite::future::block_on;

    use super::utils::{futures_array, futures_tuple, futures_vec};

    criterion_group!(
        join_benches,
        vec_join_bench,
        array_join_bench,
        tuple_join_bench
    );

    fn vec_join_bench(c: &mut Criterion) {
        c.bench_function("vec::join 10", |b| b.iter(|| vec_join(black_box(10))));
        c.bench_function("vec::join 100", |b| b.iter(|| vec_join(black_box(100))));
        c.bench_function("vec::join 1000", |b| b.iter(|| vec_join(black_box(1000))));
    }

    fn array_join_bench(c: &mut Criterion) {
        c.bench_function("array::join 10", |b| b.iter(|| array_join::<10>()));
        c.bench_function("array::join 100", |b| b.iter(|| array_join::<100>()));
        c.bench_function("array::join 1000", |b| b.iter(|| array_join::<1000>()));
    }

    fn tuple_join_bench(c: &mut Criterion) {
        c.bench_function("tuple::join 10", |b| b.iter(|| tuple_join()));
    }

    fn vec_join(max: usize) {
        block_on(async {
            let futures = futures_vec(max);
            let outputs = futures_concurrency::future::Join::join(futures).await;
            assert_eq!(outputs.len(), max);
        })
    }

    fn array_join<const N: usize>() {
        block_on(async {
            let futures = futures_array::<N>();
            let outputs = futures_concurrency::future::Join::join(futures).await;
            assert_eq!(outputs.len(), N);
        })
    }

    pub(crate) fn tuple_join() {
        block_on(async move {
            let futures = futures_tuple();
            let outputs = futures_concurrency::future::Join::join(futures).await;
            assert_eq!(outputs.0, ());
        })
    }
}

mod race {
    use criterion::{black_box, criterion_group, Criterion};
    use futures_lite::future::block_on;

    use crate::utils::futures_tuple;

    use super::utils::{futures_array, futures_vec};

    criterion_group!(
        race_benches,
        vec_race_bench,
        array_race_bench,
        tuple_race_bench
    );

    fn vec_race_bench(c: &mut Criterion) {
        c.bench_function("vec::race 10", |b| b.iter(|| vec_race(black_box(10))));
        c.bench_function("vec::race 100", |b| b.iter(|| vec_race(black_box(100))));
        c.bench_function("vec::race 1000", |b| b.iter(|| vec_race(black_box(1000))));
    }

    fn array_race_bench(c: &mut Criterion) {
        c.bench_function("array::race 10", |b| b.iter(|| array_race::<10>()));
        c.bench_function("array::race 100", |b| b.iter(|| array_race::<100>()));
        c.bench_function("array::race 1000", |b| b.iter(|| array_race::<1000>()));
    }

    fn tuple_race_bench(c: &mut Criterion) {
        c.bench_function("tuple::race 10", |b| b.iter(|| tuple_race()));
    }

    fn vec_race(max: usize) {
        block_on(async {
            let futures = futures_vec(max);
            let output = futures_concurrency::future::Race::race(futures).await;
            assert_eq!(output, ());
        })
    }

    fn array_race<const N: usize>() {
        block_on(async {
            let futures = futures_array::<N>();
            let output = futures_concurrency::future::Race::race(futures).await;
            assert_eq!(output, ());
        })
    }
    fn tuple_race() {
        block_on(async {
            let futures = futures_tuple();
            let output = futures_concurrency::future::Race::race(futures).await;
            assert_eq!(output, ());
        })
    }
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

    pub(crate) fn futures_vec(len: usize) -> Vec<CountdownFuture> {
        let wakers = Rc::new(RefCell::new(VecDeque::new()));
        let completed = Rc::new(RefCell::new(0));
        let futures: Vec<_> = (0..len)
            .map(|n| CountdownFuture::new(n, len, wakers.clone(), completed.clone()))
            .collect();
        futures
    }

    pub(crate) fn futures_array<const N: usize>() -> [CountdownFuture; N] {
        let wakers = Rc::new(RefCell::new(VecDeque::new()));
        let completed = Rc::new(RefCell::new(0));
        std::array::from_fn(|n| CountdownFuture::new(n, N, wakers.clone(), completed.clone()))
    }

    pub(crate) fn futures_tuple() -> (
        CountdownFuture,
        CountdownFuture,
        CountdownFuture,
        CountdownFuture,
        CountdownFuture,
        CountdownFuture,
        CountdownFuture,
        CountdownFuture,
        CountdownFuture,
        CountdownFuture,
    ) {
        let len = 10;
        let wakers = Rc::new(RefCell::new(VecDeque::new()));
        let completed = Rc::new(RefCell::new(0));
        (
            CountdownFuture::new(0, len, wakers.clone(), completed.clone()),
            CountdownFuture::new(1, len, wakers.clone(), completed.clone()),
            CountdownFuture::new(2, len, wakers.clone(), completed.clone()),
            CountdownFuture::new(3, len, wakers.clone(), completed.clone()),
            CountdownFuture::new(4, len, wakers.clone(), completed.clone()),
            CountdownFuture::new(5, len, wakers.clone(), completed.clone()),
            CountdownFuture::new(6, len, wakers.clone(), completed.clone()),
            CountdownFuture::new(7, len, wakers.clone(), completed.clone()),
            CountdownFuture::new(8, len, wakers.clone(), completed.clone()),
            CountdownFuture::new(9, len, wakers.clone(), completed.clone()),
        )
    }

    pub(crate) fn streams_vec(len: usize) -> Vec<CountdownStream> {
        let wakers = Rc::new(RefCell::new(VecDeque::new()));
        let completed = Rc::new(RefCell::new(0));
        let streams: Vec<_> = (0..len)
            .map(|n| CountdownStream::new(n, len, wakers.clone(), completed.clone()))
            .collect();
        streams
    }

    pub(crate) fn streams_array<const N: usize>() -> [CountdownStream; N] {
        let wakers = Rc::new(RefCell::new(VecDeque::new()));
        let completed = Rc::new(RefCell::new(0));
        std::array::from_fn(|n| CountdownStream::new(n, N, wakers.clone(), completed.clone()))
    }

    pub(crate) fn streams_tuple() -> (
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
        let len = 10;
        let wakers = Rc::new(RefCell::new(VecDeque::new()));
        let completed = Rc::new(RefCell::new(0));
        (
            CountdownStream::new(0, len, wakers.clone(), completed.clone()),
            CountdownStream::new(1, len, wakers.clone(), completed.clone()),
            CountdownStream::new(2, len, wakers.clone(), completed.clone()),
            CountdownStream::new(3, len, wakers.clone(), completed.clone()),
            CountdownStream::new(4, len, wakers.clone(), completed.clone()),
            CountdownStream::new(5, len, wakers.clone(), completed.clone()),
            CountdownStream::new(6, len, wakers.clone(), completed.clone()),
            CountdownStream::new(7, len, wakers.clone(), completed.clone()),
            CountdownStream::new(8, len, wakers.clone(), completed.clone()),
            CountdownStream::new(9, len, wakers.clone(), completed.clone()),
        )
    }

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
