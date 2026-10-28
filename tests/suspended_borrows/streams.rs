use futures::{executor::block_on, stream, StreamExt};
use futures_concurrency::stream::{Chain, Merge, Zip};

use super::support::borrow_across_pending;

mod vectors {
    use super::*;

    #[test]
    fn merge() {
        assert_eq!(
            block_on(
                vec![stream::once(borrow_across_pending())]
                    .merge()
                    .collect::<Vec<_>>()
            ),
            [Ok(2)]
        );
    }

    #[test]
    fn chain() {
        assert_eq!(
            block_on(
                vec![stream::once(borrow_across_pending())]
                    .chain()
                    .collect::<Vec<_>>()
            ),
            [Ok(2)]
        );
    }

    #[test]
    fn zip() {
        assert_eq!(
            block_on(
                vec![stream::once(borrow_across_pending())]
                    .zip()
                    .collect::<Vec<_>>()
            ),
            [vec![Ok(2)]]
        );
    }
}

mod arrays {
    use super::*;

    #[test]
    fn merge() {
        assert_eq!(
            block_on(
                [stream::once(borrow_across_pending())]
                    .merge()
                    .collect::<Vec<_>>()
            ),
            [Ok(2)]
        );
    }

    #[test]
    fn chain() {
        assert_eq!(
            block_on(
                [stream::once(borrow_across_pending())]
                    .chain()
                    .collect::<Vec<_>>()
            ),
            [Ok(2)]
        );
    }

    #[test]
    fn zip() {
        assert_eq!(
            block_on(
                [stream::once(borrow_across_pending())]
                    .zip()
                    .collect::<Vec<_>>()
            ),
            [[Ok(2)]]
        );
    }
}
