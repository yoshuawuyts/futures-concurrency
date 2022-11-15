use criterion::async_executor::FuturesExecutor;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use futures_concurrency::prelude::*;
use futures_lite::prelude::*;

mod utils;

fn vec_join(c: &mut Criterion) {
    let mut group = c.benchmark_group("compare vec::join");
    for i in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("futures-concurrency", i), i, |b, i| {
            b.to_async(FuturesExecutor).iter(|| async {
                let futs = utils::futures_vec(*i as usize);
                let output = futs.join().await;
                assert_eq!(output.len(), *i as usize);
            })
        });
        group.bench_with_input(BenchmarkId::new("futures-rs", i), i, |b, i| {
            b.to_async(FuturesExecutor).iter(|| async {
                let futs = utils::futures_vec(*i as usize);
                let output = futures::future::join_all(futs).await;
                assert_eq!(output.len(), *i as usize);
            })
        });
    }
    group.finish();
}

criterion_group!(benches, vec_join);
criterion_main!(benches);
