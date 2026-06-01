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

    fn into_unindexed(self) -> impl Producer<Item = Self::Item>
    where
        Self: Sized,
    {
        struct Adapter<P>(P);

        impl<P> Producer for Adapter<P>
        where
            P: IndexedProducer,
        {
            type Item = P::Item;

            fn into_iter(self) -> impl Iterator<Item = Self::Item> {
                self.0.into_iter()
            }

            fn split_off_left(&mut self) -> Self {
                Self(self.0.split_off_left_at(self.0.len() / 2))
            }
        }

        Adapter(self)
    }
}
