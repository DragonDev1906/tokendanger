use std::ops::Range;

#[derive(Debug)]
pub struct DynChunkIter {
    pos: usize,
    chunk_size: usize,
    target_amount: usize,
}

impl Iterator for DynChunkIter {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.pos..(self.pos + self.chunk_size);
        self.pos += self.chunk_size;
        Some(chunk)
    }
}

impl DynChunkIter {
    pub fn new(start: usize, initial_chunk_size: usize, target_amount: usize) -> Self {
        Self {
            pos: start,
            chunk_size: initial_chunk_size,
            target_amount,
        }
    }

    /// Update the size of future chunks such that they are closer to the target
    /// amount.
    ///
    /// Note that this does NOT average over multiple chunks and only takes tha
    /// last one into account and is thus subject to big changes if there is a
    /// sudden big change in the amount/chunk.
    pub fn update_chunk_size(&mut self, amount: usize) {
        // Don't do anything if the amount is 0
        if amount > 0 {
            self.chunk_size = self.chunk_size * self.target_amount / amount;
        }
    }

    pub fn half_chunk_size(&mut self) {
        self.chunk_size = self.chunk_size >> 1;
    }
}
