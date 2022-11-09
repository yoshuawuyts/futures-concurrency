// use futures_concurrency::prelude::*;
// use futures_core::Stream;
// use futures_lite::future::block_on;
// use futures_lite::prelude::*;
// use pin_project::pin_project;

// use std::cell::RefCell;
// use std::pin::Pin;
// use std::rc::Rc;
// use std::task::{Context, Poll};

// pub(crate) fn merge_test(max: usize) {
//     block_on(async {
//         let counter = Rc::new(RefCell::new(max));
//         let futures: Vec<_> = (1..=max)
//             .rev()
//             .map(|n| Countdown::new(n, counter.clone()))
//             .collect();
//         let mut s = futures.merge();

//         let mut counter = 0;
//         while let Some(_) = s.next().await {
//             counter += 1;
//         }
//         assert_eq!(counter, max);
//     })
// }

// /// A future which will _eventually_ be ready, but needs to be polled N times before it is.
// #[pin_project]
// struct Countdown {
//     success_count: Rc<RefCell<usize>>,
//     target_count: usize,
//     done: bool,
// }

// impl Countdown {
//     fn new(count: usize, success_count: Rc<RefCell<usize>>) -> Self {
//         Self {
//             success_count,
//             target_count: count,
//             done: false,
//         }
//     }
// }
// impl Stream for Countdown {
//     type Item = ();

//     fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         let this = self.project();
//         if *this.done {
//             Poll::Ready(None)
//         } else if *this.success_count.borrow() == *this.target_count {
//             *this.success_count.borrow_mut() -= 1;
//             *this.done = true;
//             Poll::Ready(Some(()))
//         } else {
//             Poll::Pending
//         }
//     }
// }

// #[cfg(test)]
// mod test {
//     use super::*;
//     #[test]
//     fn smoke() {
//         merge_test(4);
//     }
// }
