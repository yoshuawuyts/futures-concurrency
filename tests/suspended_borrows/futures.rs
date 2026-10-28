use std::{
    cell::Cell,
    future::{pending, poll_fn, Future},
    pin::pin,
    task::{Context, Poll},
};

use futures::{executor::block_on, task::noop_waker};
use futures_concurrency::future::{Join, Race, RaceOk, TryJoin};

use super::support::{borrow_across_pending, borrow_until_drop, DebugFuture};

mod vectors {
    use super::*;

    #[test]
    fn join() {
        assert_eq!(block_on(vec![borrow_across_pending()].join()), [Ok(2)]);
    }

    #[test]
    fn try_join() {
        assert_eq!(
            block_on(vec![borrow_across_pending()].try_join()),
            Ok(vec![2])
        );
    }

    #[test]
    fn race() {
        assert_eq!(block_on(vec![borrow_across_pending()].race()), Ok(2));
    }

    #[test]
    fn race_ok() {
        assert_eq!(
            block_on(vec![borrow_across_pending()].race_ok()).ok(),
            Some(2)
        );
    }

    #[test]
    fn debug_while_pending() {
        let mut future = pin!(vec![DebugFuture::new(borrow_across_pending())].race());
        let waker = noop_waker();
        let mut context = Context::from_waker(&waker);
        assert!(future.as_mut().poll(&mut context).is_pending());
        assert!(!format!("{future:?}").is_empty());
        assert_eq!(future.as_mut().poll(&mut context), Poll::Ready(Ok(2)));
    }

    #[test]
    fn cancellation() {
        let drops = Cell::new(0);
        {
            let mut future =
                pin!(vec![borrow_until_drop(&drops), borrow_until_drop(&drops)].try_join());
            let waker = noop_waker();
            assert!(future
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_pending());
        }
        assert_eq!(drops.get(), 2);
    }

    #[test]
    fn later_error() {
        let futures = (0..64)
            .map(|index| async move {
                if index == 63 {
                    Err(17)
                } else {
                    pending::<Result<(), u8>>().await
                }
            })
            .collect::<Vec<_>>();
        let mut future = pin!(futures.try_join());
        let waker = noop_waker();
        assert_eq!(
            future.as_mut().poll(&mut Context::from_waker(&waker)),
            Poll::Ready(Err(17))
        );
    }

    #[test]
    fn input_order() {
        let futures = (0..8)
            .map(|index| async move {
                let mut polls = 0;
                poll_fn(|context| {
                    polls += 1;
                    if polls == 8 - index {
                        Poll::Ready(Ok::<_, ()>(index))
                    } else {
                        context.waker().wake_by_ref();
                        Poll::Pending
                    }
                })
                .await
            })
            .collect::<Vec<_>>();
        assert_eq!(block_on(futures.try_join()), Ok((0..8).collect::<Vec<_>>()));
    }

    #[test]
    fn empty_join() {
        let futures = Vec::<std::future::Ready<Result<(), ()>>>::new();
        assert_eq!(block_on(futures.try_join()), Ok(vec![]));
    }

    #[test]
    fn completed_box() {
        struct OwnsBox {
            value: Box<usize>,
            pending: bool,
        }

        impl Future for OwnsBox {
            type Output = Result<usize, ()>;

            fn poll(
                mut self: std::pin::Pin<&mut Self>,
                context: &mut Context<'_>,
            ) -> Poll<Self::Output> {
                if self.pending {
                    self.pending = false;
                    context.waker().wake_by_ref();
                    Poll::Pending
                } else {
                    Poll::Ready(Ok(*self.value))
                }
            }
        }

        let futures = vec![
            OwnsBox {
                value: Box::new(1),
                pending: false,
            },
            OwnsBox {
                value: Box::new(2),
                pending: true,
            },
        ];
        assert_eq!(block_on(futures.try_join()), Ok(vec![1, 2]));
    }
}

mod arrays {
    use super::*;

    #[test]
    fn join() {
        assert_eq!(block_on([borrow_across_pending()].join()), [Ok(2)]);
    }

    #[test]
    fn try_join() {
        assert_eq!(block_on([borrow_across_pending()].try_join()), Ok([2]));
    }

    #[test]
    fn race() {
        assert_eq!(block_on([borrow_across_pending()].race()), Ok(2));
    }

    #[test]
    fn race_ok() {
        assert_eq!(block_on([borrow_across_pending()].race_ok()).ok(), Some(2));
    }

    #[test]
    fn debug_while_pending() {
        let mut future = pin!([DebugFuture::new(borrow_across_pending())].race());
        let waker = noop_waker();
        let mut context = Context::from_waker(&waker);
        assert!(future.as_mut().poll(&mut context).is_pending());
        assert!(!format!("{future:?}").is_empty());
        assert_eq!(future.as_mut().poll(&mut context), Poll::Ready(Ok(2)));
    }

    #[test]
    fn cancellation() {
        let drops = Cell::new(0);
        {
            let mut future =
                pin!([borrow_until_drop(&drops), borrow_until_drop(&drops)].try_join());
            let waker = noop_waker();
            assert!(future
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_pending());
        }
        assert_eq!(drops.get(), 2);
    }
}

#[test]
fn tuple_join() {
    assert_eq!(
        block_on((borrow_across_pending(), borrow_across_pending()).try_join()),
        Ok((2, 2))
    );
}
