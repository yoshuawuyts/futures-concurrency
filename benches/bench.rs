#![allow(clippy::let_unit_value, clippy::unit_cmp)]

mod utils;

criterion::criterion_main!(merge::merge_benches, join::join_benches, race::race_benches);

mod merge {
    use criterion::async_executor::FuturesExecutor;
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
        c.bench_function("vec::merge 10", |b| {
            b.to_async(FuturesExecutor)
                .iter(|| vec_merge(black_box(10)))
        });
        c.bench_function("vec::merge 100", |b| {
            b.to_async(FuturesExecutor)
                .iter(|| vec_merge(black_box(100)))
        });
        c.bench_function("vec::merge 1000", |b| {
            b.to_async(FuturesExecutor)
                .iter(|| vec_merge(black_box(1000)))
        });
    }

    fn array_merge_bench(c: &mut Criterion) {
        c.bench_function("array::merge 10", |b| {
            b.to_async(FuturesExecutor).iter(array_merge::<10>)
        });
        c.bench_function("array::merge 100", |b| {
            b.to_async(FuturesExecutor).iter(array_merge::<100>)
        });
        c.bench_function("array::merge 1000", |b| {
            b.to_async(FuturesExecutor).iter(array_merge::<1000>)
        });
    }

    fn tuple_merge_bench(c: &mut Criterion) {
        c.bench_function("tuple::merge 10", |b| {
            b.to_async(FuturesExecutor).iter(tuple_merge)
        });
    }

    async fn vec_merge(max: usize) {
        let mut counter = 0;
        let streams = streams_vec(max);
        let mut s = streams.merge();
        while s.next().await.is_some() {
            counter += 1;
        }
        assert_eq!(counter, max);
    }

    async fn array_merge<const N: usize>() {
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

    async fn tuple_merge() {
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
    use criterion::async_executor::FuturesExecutor;
    use criterion::{black_box, criterion_group, Criterion};
    use futures_concurrency::prelude::*;

    use super::utils::{futures_array, futures_tuple, futures_vec};

    criterion_group!(
        join_benches,
        vec_join_bench,
        array_join_bench,
        tuple_join_bench
    );

    fn vec_join_bench(c: &mut Criterion) {
        c.bench_function("vec::join 10", move |b| {
            b.to_async(FuturesExecutor).iter(|| vec_join(black_box(10)))
        });
        c.bench_function("vec::join 100", |b| {
            b.to_async(FuturesExecutor)
                .iter(|| vec_join(black_box(100)))
        });
        c.bench_function("vec::join 1000", |b| {
            b.to_async(FuturesExecutor)
                .iter(|| vec_join(black_box(1000)))
        });
    }

    fn array_join_bench(c: &mut Criterion) {
        c.bench_function("array::join 10", |b| {
            b.to_async(FuturesExecutor).iter(array_join::<10>)
        });
        c.bench_function("array::join 100", |b| {
            b.to_async(FuturesExecutor).iter(array_join::<100>)
        });
        c.bench_function("array::join 1000", |b| {
            b.to_async(FuturesExecutor).iter(array_join::<1000>)
        });
    }

    fn tuple_join_bench(c: &mut Criterion) {
        c.bench_function("tuple::join 10", |b| {
            b.to_async(FuturesExecutor).iter(tuple_join)
        });
    }

    async fn vec_join(max: usize) {
        let futures = futures_vec(max);
        let output = futures.join().await;
        assert_eq!(output.len(), max);
    }

    async fn array_join<const N: usize>() {
        let futures = futures_array::<N>();
        let output = futures.join().await;
        assert_eq!(output.len(), N);
    }

    async fn tuple_join() {
        let futures = futures_tuple();
        let output = futures.join().await;
        assert_eq!(output.0, ());
    }
}

mod race {
    use criterion::async_executor::FuturesExecutor;
    use criterion::{black_box, criterion_group, Criterion};
    use futures_concurrency::prelude::*;

    use crate::utils::futures_tuple;

    use super::utils::{futures_array, futures_vec};

    criterion_group!(
        race_benches,
        vec_race_bench,
        array_race_bench,
        tuple_race_bench
    );

    fn vec_race_bench(c: &mut Criterion) {
        c.bench_function("vec::race 10", |b| {
            b.to_async(FuturesExecutor).iter(|| vec_race(black_box(10)))
        });
        c.bench_function("vec::race 100", |b| {
            b.to_async(FuturesExecutor)
                .iter(|| vec_race(black_box(100)))
        });
        c.bench_function("vec::race 1000", |b| {
            b.to_async(FuturesExecutor)
                .iter(|| vec_race(black_box(1000)))
        });
    }

    fn array_race_bench(c: &mut Criterion) {
        c.bench_function("array::race 10", |b| {
            b.to_async(FuturesExecutor).iter(array_race::<10>)
        });
        c.bench_function("array::race 100", |b| {
            b.to_async(FuturesExecutor).iter(array_race::<100>)
        });
        c.bench_function("array::race 1000", |b| {
            b.to_async(FuturesExecutor).iter(array_race::<1000>)
        });
    }

    fn tuple_race_bench(c: &mut Criterion) {
        c.bench_function("tuple::race 10", |b| {
            b.to_async(FuturesExecutor).iter(tuple_race)
        });
    }

    async fn vec_race(max: usize) {
        let futures = futures_vec(max);
        let output = futures.race().await;
        assert_eq!(output, ());
    }

    async fn array_race<const N: usize>() {
        let futures = futures_array::<N>();
        let output = futures.race().await;
        assert_eq!(output, ());
    }

    async fn tuple_race() {
        let futures = futures_tuple();
        let output = futures.race().await;
        assert_eq!(output, ());
    }
}
