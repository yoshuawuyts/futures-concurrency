use super::{ConcurrentStream, Consumer};
use std::{future::Future, marker::PhantomData};

/// Convert items from one type into another
#[derive(Debug)]
pub struct Map<CS, F, Fut, T, B>
where
    CS: ConcurrentStream,
    F: Fn(T) -> Fut,
    Fut: Future<Output = B>,
{
    inner: CS,
    f: F,
    _phantom: PhantomData<(Fut, T, B)>,
}

impl<CS, F, Fut, T, B> Map<CS, F, Fut, T, B>
where
    CS: ConcurrentStream<Item = T>,
    F: Fn(T) -> Fut,
    Fut: Future<Output = B>,
{
    pub(crate) fn new(inner: CS, f: F) -> Self {
        Self {
            inner,
            f,
            _phantom: PhantomData,
        }
    }
}

impl<CS, F, Fut, T, B> ConcurrentStream for Map<CS, F, Fut, T, B>
where
    CS: ConcurrentStream<Item = T>,
    F: Fn(T) -> Fut,
    Fut: Future<Output = B>,
{
    type Item = B;

    async fn drive<C>(self, consumer: C) -> C::Output
    where
        C: Consumer<Self::Item>,
    {
        let consumer = MapConsumer {
            inner: consumer,
            f: self.f,
            _phantom: PhantomData,
        };
        self.inner.drive(consumer).await
    }
}

// OK: validated! - all bounds should check out
struct MapConsumer<C, F, Fut, T, B>
where
    C: Consumer<B>,
    F: Fn(T) -> Fut,
    Fut: Future<Output = B>,
{
    inner: C,
    f: F,
    _phantom: PhantomData<(Fut, T, B)>,
}

// OK: validated! - we push types `B` into the next consumer
impl<C, F, Fut, T, B> Consumer<T> for MapConsumer<C, F, Fut, T, B>
where
    C: Consumer<B>,
    F: Fn(T) -> Fut,
    Fut: Future<Output = B>,
{
    type Output = C::Output;

    async fn send<Fut2: Future<Output = T>>(&mut self, future: Fut2) {
        self.inner
            .send(Box::pin(async {
                let t = future.await;
                (self.f)(t).await
            }))
            .await;
    }

    async fn finish(self) -> Self::Output {
        self.inner.finish().await
    }
}
