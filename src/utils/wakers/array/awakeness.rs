use super::super::dummy_waker;

use core::task::Waker;

pub(crate) struct AwakenessArray<const N: usize> {
    awake_set: [bool; N],
    awake_list: [usize; N],
    awake_list_len: usize,
    parent_waker: Waker,
}

impl<const N: usize> AwakenessArray<N> {
    pub(crate) fn new() -> Self {
        Self {
            awake_set: [true; N],
            awake_list: core::array::from_fn(core::convert::identity),
            awake_list_len: N,
            parent_waker: dummy_waker(),
        }
    }
    pub(crate) fn set_parent_waker(&mut self, waker: &Waker) {
        self.parent_waker = waker.to_owned();
    }
    fn set_woken(&mut self, index: usize) -> bool {
        let was_awake = std::mem::replace(&mut self.awake_set[index], true);
        if !was_awake {
            self.awake_list[self.awake_list_len] = index;
            self.awake_list_len += 1;
        }
        was_awake
    }
    pub(crate) fn wake(&mut self, index: usize) {
        if !self.set_woken(index) && self.awake_list_len == 1 {
            self.parent_waker.wake_by_ref();
        }
    }
    pub(crate) fn awake_list(&self) -> &[usize] {
        &self.awake_list[..self.awake_list_len]
    }
    const TRESHOLD: usize = N / 64;
    pub(crate) fn clear(&mut self) {
        if self.awake_list_len < Self::TRESHOLD {
            self.awake_set.fill(false);
        } else {
            self.awake_list.iter().for_each(|&idx| {
                self.awake_set[idx] = false;
            });
        }
        self.awake_list_len = 0;
    }
}
