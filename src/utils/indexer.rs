use core::ops;

/// Generate an iteration sequence. This provides *fair* iteration when multiple
/// futures need to be polled concurrently.
pub(crate) struct Indexer {
    offset: usize,
    max: usize,
}

impl Indexer {
    pub(crate) fn new(max: usize) -> Self {
        Self { offset: 0, max }
    }

    /// Generate a range between `0..max`, incrementing the starting point
    /// for the next iteration.
    pub(crate) fn iter(&mut self) -> IndexIter {
        // Increment the starting point for next time.
        let offset = self.offset;
        self.offset = (self.offset + 1).wrapping_rem(self.max);

        IndexIter {
            iter: (0..self.max),
            offset,
        }
    }
}

pub(crate) struct IndexIter {
    iter: ops::Range<usize>,
    offset: usize,
}

impl Iterator for IndexIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|pos| (pos + self.offset).wrapping_rem(self.iter.end))
    }
}
