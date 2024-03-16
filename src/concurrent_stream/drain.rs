use super::Consumer;
use core::future::Future;

pub(crate) struct Drain;

impl<Item, Fut> Consumer<Item, Fut> for Drain
where
    Fut: Future<Output = Item>,
{
    type Output = ();

    async fn send(&mut self, future: Fut) {
        future.await;
    }

    async fn progress(&mut self) {}
    async fn finish(self) -> Self::Output {}
}
