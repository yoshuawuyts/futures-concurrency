use std::{cell::Cell, pin::pin, task::Context};

use futures::{executor::block_on, stream, task::noop_waker, Stream, StreamExt};
use futures_concurrency::{future::FutureGroup, stream::StreamGroup};

use super::support::{borrow_across_pending, borrow_until_drop};

mod future_group {
    use super::*;

    #[test]
    fn resume() {
        let mut group = FutureGroup::new();
        for _ in 0..3 {
            group.insert(borrow_across_pending());
        }
        assert_eq!(block_on(group.collect::<Vec<_>>()), [Ok(2); 3]);
    }

    #[test]
    fn cancellation() {
        let drops = Cell::new(0);
        {
            let mut group = FutureGroup::new();
            for _ in 0..3 {
                group.insert(borrow_until_drop(&drops));
            }
            let mut group = pin!(group);
            let waker = noop_waker();
            assert!(group
                .as_mut()
                .poll_next(&mut Context::from_waker(&waker))
                .is_pending());
        }
        assert_eq!(drops.get(), 3);
    }
}

mod stream_group {
    use super::*;

    #[test]
    fn resume() {
        let mut group = StreamGroup::new();
        for _ in 0..3 {
            group.insert(stream::once(borrow_across_pending()));
        }
        assert_eq!(block_on(group.collect::<Vec<_>>()), [Ok(2); 3]);
    }

    #[test]
    fn cancellation() {
        let drops = Cell::new(0);
        {
            let mut group = StreamGroup::new();
            for _ in 0..3 {
                group.insert(stream::once(borrow_until_drop(&drops)));
            }
            let mut group = pin!(group);
            let waker = noop_waker();
            assert!(group
                .as_mut()
                .poll_next(&mut Context::from_waker(&waker))
                .is_pending());
        }
        assert_eq!(drops.get(), 3);
    }
}
