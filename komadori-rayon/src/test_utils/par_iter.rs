mod cloned;

pub use cloned::Cloned;

use super::{IndexedProducer, Producer};

/// Thread-pool-agnostic parallel iterator.
///
/// It is not aware of the exact amount of items.
pub trait ParallelIterator {
    type Item;

    fn producer(&mut self) -> impl Producer<Item = Self::Item>;

    fn cloned(self) -> Cloned<Self>
    where
        Self: Sized,
    {
        Cloned::new(self)
    }
}

/// The indexed version of the thread-pool-agnostic parallel iterator.
pub trait IndexedParallelIterator: ParallelIterator {
    fn indexed_producer(&mut self) -> impl IndexedProducer<Item = Self::Item>;
    fn len(&self) -> usize;
}

pub trait IntoParallelIterator {
    type Item;

    type IntoParIter: ParallelIterator;

    fn into_par_iter(self) -> Self::IntoParIter;
}

pub trait ParallelIteratorByRef
where
    for<'a> &'a Self: IntoParallelIterator,
{
    fn par_iter(&self) -> <&'_ Self as IntoParallelIterator>::IntoParIter {
        self.into_par_iter()
    }
}
impl<T> ParallelIteratorByRef for T
where
    T: ?Sized,
    for<'a> &'a T: IntoParallelIterator,
{
}
