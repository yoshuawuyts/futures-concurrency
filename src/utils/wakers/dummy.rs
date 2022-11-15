use std::{sync::Arc, task::Wake};

pub(crate) struct DummyWaker();
impl Wake for DummyWaker {
    fn wake(self: Arc<Self>) {}
}
