use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

pub(crate) mod array;

fn poll_next_in_order<F, S, I>(
    first: Pin<&mut F>,
    second: Pin<&mut S>,
    cx: &mut Context<'_>,
) -> Poll<Option<I>>
where
    F: Stream<Item = I>,
    S: Stream<Item = I>,
{
    match first.poll_next(cx) {
        Poll::Ready(None) => second.poll_next(cx),
        Poll::Ready(item) => Poll::Ready(item),
        Poll::Pending => match second.poll_next(cx) {
            Poll::Ready(None) | Poll::Pending => Poll::Pending,
            Poll::Ready(item) => Poll::Ready(item),
        },
    }
}

/// Generates a random number in `0..n`.
pub(crate) fn random(n: u32) -> u32 {
    use std::cell::Cell;
    use std::num::Wrapping;

    thread_local! {
        static RNG: Cell<Wrapping<u32>> = {
            // Take the address of a local value as seed.
            let mut x = 0i32;
            let r = &mut x;
            let addr = r as *mut i32 as usize;
            Cell::new(Wrapping(addr as u32))
        }
    }

    RNG.with(|rng| {
        // This is the 32-bit variant of Xorshift.
        //
        // Source: https://en.wikipedia.org/wiki/Xorshift
        let mut x = rng.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        rng.set(x);

        // This is a fast alternative to `x % n`.
        //
        // Author: Daniel Lemire
        // Source: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/
        ((u64::from(x.0)).wrapping_mul(u64::from(n)) >> 32) as u32
    })
}
