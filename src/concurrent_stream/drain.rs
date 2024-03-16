use super::Consumer;
use core::future::Future;

pub(crate) struct Drain;

impl<Item> Consumer<Item> for Drain {
    type Output = ();

    async fn send<Fut: Future>(&mut self, future: Fut) {
        future.await;
    }

    async fn finish(self) -> Self::Output {
        ()
    }
}
