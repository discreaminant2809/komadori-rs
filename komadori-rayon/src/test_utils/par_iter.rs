mod chain;
mod cloned;
mod filter;

pub use chain::*;
pub use cloned::*;
pub use filter::*;

use super::{IndexedProducer, Producer};

/// Thread-pool-agnostic parallel iterator.
///
/// It is not aware of the exact amount of items.
pub trait ParallelIterator {
    type Item;

    fn take_producer(&mut self) -> impl Producer<Item = Self::Item>;

    fn cloned(self) -> Cloned<Self>
    where
        Self: Sized,
    {
        Cloned::new(self)
    }

    fn filter<F>(self, f: F) -> Filter<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Item) -> bool,
    {
        Filter::new(self, f)
    }

    fn chain<I>(self, other: I) -> Chain<Self, I::IntoParIter>
    where
        Self: Sized,
        I: IntoParallelIterator<Item = Self::Item>,
    {
        Chain::new(self, other.into_par_iter())
    }

    fn take_iter(&mut self) -> impl Iterator<Item = Self::Item> {
        self.take_producer().into_iter()
    }

    /// # Notes
    ///
    /// Not executed in parallel.
    fn count(self) -> usize
    where
        Self: Sized,
    {
        let mut this = self;
        this.take_iter().count()
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

impl<I> IntoParallelIterator for I
where
    I: ParallelIterator,
{
    type Item = I::Item;

    type IntoParIter = Self;

    fn into_par_iter(self) -> Self::IntoParIter {
        self
    }
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
