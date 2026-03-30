pub trait Producer {
    type Item;

    fn into_iter(self) -> impl Iterator<Item = Self::Item>;

    fn split_off_left(&mut self) -> Self;
}

pub trait IndexedProducer {
    type Item;

    fn into_iter(self) -> impl Iterator<Item = Self::Item>;

    fn len(&self) -> usize;

    fn split_off_left_at(&mut self, index: usize) -> Self;
}
