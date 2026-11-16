use crate::collections::{Append, CollectionBase};

impl CollectionBase for String {
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.capacity()
    }
}

impl Append for String {
    fn append(&mut self, other: Self) {
        *self += &other;
    }
}
