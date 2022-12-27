pub(crate) struct ArrayDequeue<T, const N: usize> {
    data: [T; N],
    start: usize,
    len: usize,
}

impl<T, const N: usize> ArrayDequeue<T, N> {
    pub(crate) fn new(data: [T; N], len: usize) -> Self {
        Self {
            data,
            start: 0,
            len,
        }
    }
    pub(crate) fn push_back(&mut self, elem: T) {
        assert!(self.len < N, "array is full");
        self.data[(self.start + self.len) % N] = elem;
        self.len += 1;
    }
}

struct ArrayDequeueDrain<'a, T: Copy, const N: usize> {
    arr: &'a mut ArrayDequeue<T, N>,
}
impl<'a, T: Copy, const N: usize> Iterator for ArrayDequeueDrain<'a, T, N> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.arr.len > 0 {
            let elem = self.arr.data[self.arr.start];
            self.arr.start = (self.arr.start + 1) % N;
            self.arr.len -= 1;
            Some(elem)
        } else {
            None
        }
    }
}

impl<T: Copy, const N: usize> ArrayDequeue<T, N> {
    pub(crate) fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        ArrayDequeueDrain { arr: self }
    }
}
impl<T, const N: usize> Extend<T> for ArrayDequeue<T, N> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        iter.into_iter().for_each(|elem| {
            self.push_back(elem);
        });
    }
}
