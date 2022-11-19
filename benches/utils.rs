#![allow(unused)]

use futures_core::Stream;
use futures_lite::prelude::*;
use pin_project::pin_project;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

pub fn futures_vec(len: usize) -> Vec<CountdownFuture> {
    let wakers = Rc::new(RefCell::new(VecDeque::new()));
    let completed = Rc::new(RefCell::new(0));
    let futures: Vec<_> = (0..len)
        .map(|n| CountdownFuture::new(n, len, wakers.clone(), completed.clone()))
        .collect();
    futures
}

pub fn futures_array<const N: usize>() -> [CountdownFuture; N] {
    let wakers = Rc::new(RefCell::new(VecDeque::new()));
    let completed = Rc::new(RefCell::new(0));
    std::array::from_fn(|n| CountdownFuture::new(n, N, wakers.clone(), completed.clone()))
}

pub fn futures_tuple() -> (
    CountdownFuture,
    CountdownFuture,
    CountdownFuture,
    CountdownFuture,
    CountdownFuture,
    CountdownFuture,
    CountdownFuture,
    CountdownFuture,
    CountdownFuture,
    CountdownFuture,
) {
    let len = 10;
    let wakers = Rc::new(RefCell::new(VecDeque::new()));
    let completed = Rc::new(RefCell::new(0));
    (
        CountdownFuture::new(0, len, wakers.clone(), completed.clone()),
        CountdownFuture::new(1, len, wakers.clone(), completed.clone()),
        CountdownFuture::new(2, len, wakers.clone(), completed.clone()),
        CountdownFuture::new(3, len, wakers.clone(), completed.clone()),
        CountdownFuture::new(4, len, wakers.clone(), completed.clone()),
        CountdownFuture::new(5, len, wakers.clone(), completed.clone()),
        CountdownFuture::new(6, len, wakers.clone(), completed.clone()),
        CountdownFuture::new(7, len, wakers.clone(), completed.clone()),
        CountdownFuture::new(8, len, wakers.clone(), completed.clone()),
        CountdownFuture::new(9, len, wakers, completed),
    )
}

pub fn streams_vec(len: usize) -> Vec<CountdownStream> {
    let wakers = Rc::new(RefCell::new(VecDeque::new()));
    let completed = Rc::new(RefCell::new(0));
    let streams: Vec<_> = (0..len)
        .map(|n| CountdownStream::new(n, len, wakers.clone(), completed.clone()))
        .collect();
    streams
}

pub fn streams_array<const N: usize>() -> [CountdownStream; N] {
    let wakers = Rc::new(RefCell::new(VecDeque::new()));
    let completed = Rc::new(RefCell::new(0));
    std::array::from_fn(|n| CountdownStream::new(n, N, wakers.clone(), completed.clone()))
}

pub fn streams_tuple() -> (
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
    CountdownStream,
) {
    let len = 10;
    let wakers = Rc::new(RefCell::new(VecDeque::new()));
    let completed = Rc::new(RefCell::new(0));
    (
        CountdownStream::new(0, len, wakers.clone(), completed.clone()),
        CountdownStream::new(1, len, wakers.clone(), completed.clone()),
        CountdownStream::new(2, len, wakers.clone(), completed.clone()),
        CountdownStream::new(3, len, wakers.clone(), completed.clone()),
        CountdownStream::new(4, len, wakers.clone(), completed.clone()),
        CountdownStream::new(5, len, wakers.clone(), completed.clone()),
        CountdownStream::new(6, len, wakers.clone(), completed.clone()),
        CountdownStream::new(7, len, wakers.clone(), completed.clone()),
        CountdownStream::new(8, len, wakers.clone(), completed.clone()),
        CountdownStream::new(9, len, wakers, completed),
    )
}

#[derive(Clone, Copy)]
enum State {
    Init,
    Polled,
    Done,
}

/// A stream which will _eventually_ be ready, but needs to be polled N times before it is.
#[pin_project]
pub struct CountdownStream {
    state: State,
    wakers: Rc<RefCell<VecDeque<Waker>>>,
    index: usize,
    max_count: usize,
    completed_count: Rc<RefCell<usize>>,
}

impl CountdownStream {
    pub fn new(
        index: usize,
        max_count: usize,
        wakers: Rc<RefCell<VecDeque<Waker>>>,
        completed_count: Rc<RefCell<usize>>,
    ) -> Self {
        Self {
            state: State::Init,
            wakers,
            max_count,
            index,
            completed_count,
        }
    }
}
impl Stream for CountdownStream {
    type Item = ();

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        // If we are the last stream to be polled, skip strait to the Polled state.
        if this.wakers.borrow().len() + 1 == *this.max_count {
            *this.state = State::Polled;
        }

        match this.state {
            State::Init => {
                // Push our waker onto the stack so we get woken again someday.
                this.wakers.borrow_mut().push_back(cx.waker().clone());
                *this.state = State::Polled;
                Poll::Pending
            }
            State::Polled => {
                // Wake up the next one
                let _ = this.wakers.borrow_mut().pop_front().map(Waker::wake);

                if *this.completed_count.borrow() == *this.index {
                    *this.state = State::Done;
                    *this.completed_count.borrow_mut() += 1;
                    Poll::Ready(Some(()))
                } else {
                    // We're not done yet, so schedule another wakeup
                    this.wakers.borrow_mut().push_back(cx.waker().clone());
                    Poll::Pending
                }
            }
            State::Done => Poll::Ready(None),
        }
    }
}

/// A future which will _eventually_ be ready, but needs to be polled N times before it is.
#[pin_project]
pub struct CountdownFuture {
    state: State,
    wakers: Rc<RefCell<VecDeque<Waker>>>,
    index: usize,
    max_count: usize,
    completed_count: Rc<RefCell<usize>>,
}

impl CountdownFuture {
    pub fn new(
        index: usize,
        max_count: usize,
        wakers: Rc<RefCell<VecDeque<Waker>>>,
        completed_count: Rc<RefCell<usize>>,
    ) -> Self {
        Self {
            state: State::Init,
            wakers,
            max_count,
            index,
            completed_count,
        }
    }
}
impl Future for CountdownFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        // If we are the last stream to be polled, skip strait to the Polled state.
        if this.wakers.borrow().len() + 1 == *this.max_count {
            *this.state = State::Polled;
        }

        match this.state {
            State::Init => {
                // Push our waker onto the stack so we get woken again someday.
                this.wakers.borrow_mut().push_back(cx.waker().clone());
                *this.state = State::Polled;
                Poll::Pending
            }
            State::Polled => {
                // Wake up the next one
                let _ = this.wakers.borrow_mut().pop_front().map(Waker::wake);

                if *this.completed_count.borrow() == *this.index {
                    *this.state = State::Done;
                    *this.completed_count.borrow_mut() += 1;
                    Poll::Ready(())
                } else {
                    // We're not done yet, so schedule another wakeup
                    this.wakers.borrow_mut().push_back(cx.waker().clone());
                    Poll::Pending
                }
            }
            State::Done => Poll::Ready(()),
        }
    }
}
