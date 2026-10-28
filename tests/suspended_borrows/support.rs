use std::{
    cell::Cell,
    fmt,
    future::{poll_fn, Future},
    pin::Pin,
    task::{Context, Poll},
};

#[pin_project::pin_project]
pub struct DebugFuture<F> {
    #[pin]
    future: F,
}

impl<F> DebugFuture<F> {
    pub fn new(future: F) -> Self {
        Self { future }
    }
}

impl<F: Future> Future for DebugFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().future.poll(context)
    }
}

impl<F> fmt::Debug for DebugFuture<F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DebugFuture")
    }
}

pub async fn borrow_across_pending() -> Result<usize, ()> {
    let mut value = 0;
    let reference = &mut value;
    poll_fn(|context| {
        *reference += 1;
        if *reference == 1 {
            context.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(Ok(*reference))
        }
    })
    .await
}

struct DropBorrow<'a> {
    value: &'a mut usize,
    drops: &'a Cell<usize>,
}

impl Drop for DropBorrow<'_> {
    fn drop(&mut self) {
        *self.value += 1;
        self.drops.set(self.drops.get() + 1);
    }
}

pub async fn borrow_until_drop(drops: &Cell<usize>) -> Result<(), ()> {
    let mut value = 0;
    let guard = DropBorrow {
        value: &mut value,
        drops,
    };
    poll_fn(|_| {
        *guard.value += 1;
        Poll::<Result<(), ()>>::Pending
    })
    .await
}
