use super::ConcurrentStream;
use core::future::Future;
use core::pin::Pin;

/// An iterator that maps value of another stream with a function.
#[derive(Debug)]
pub struct Map<I, F> {
    stream: I,
    f: F,
}

impl<I, F> Map<I, F> {
    pub(crate) fn new(stream: I, f: F) -> Self {
        Self { stream, f }
    }
}

impl<I, F, B, Fut> ConcurrentStream for Map<I, F>
where
    I: ConcurrentStream,
    F: Fn(I::Item) -> Fut,
    Fut: Future<Output = B>,
{
    type Item = B;
    // TODO: hand-roll this future
    type Future<'a> = Pin<Box<dyn Future<Output = Option<Self::Item>> + 'a>> where Self: 'a;
    fn next(&self) -> Self::Future<'_> {
        let fut = self.stream.next();
        Box::pin(async {
            let item = fut.await?;
            let out = (self.f)(item).await;
            Some(out)
        })
    }
}
