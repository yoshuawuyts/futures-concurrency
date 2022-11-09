use criterion::{black_box, criterion_group, criterion_main, Criterion};
use futures_concurrency::prelude::*;
use futures_core::Stream;
use futures_lite::future::block_on;
use futures_lite::prelude::*;
use pin_project::pin_project;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("merge 10", |b| b.iter(|| merge_test(black_box(10))));
    // c.bench_function("merge 100", |b| b.iter(|| merge_futures(black_box(100))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

pub(crate) fn merge_test(max: usize) {
    block_on(async {
        let futures: Vec<_> = (0..max).rev().map(Countdown::new).collect();
        let mut s = futures.merge();

        let mut counter = 0;
        while let Some(_) = s.next().await {
            counter += 1;
        }
        assert_eq!(counter, max);
    })
}

/// A future which will _eventually_ be ready, but needs to be polled N times before it is.
#[pin_project]
struct Countdown {
    count: usize,
    done: bool,
}

impl Countdown {
    fn new(count: usize) -> Self {
        Self { count, done: false }
    }
}
impl Stream for Countdown {
    type Item = ();

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        if *this.done {
            Poll::Ready(None)
        } else if *this.count == 0 {
            *this.done = true;
            Poll::Ready(Some(()))
        } else {
            *this.count -= 1;
            Poll::Pending
        }
    }
}

impl Future for Countdown {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        if *this.done {
            panic!("futures should not be polled after completing");
        } else if *this.count == 0 {
            *this.done = true;
            Poll::Ready(())
        } else {
            *this.count -= 1;
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn smoke() {
        merge_test(3);
    }
}
