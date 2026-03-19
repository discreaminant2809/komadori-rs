//!

use std::ops::ControlFlow;

use crate::{
    collections::reservable::with_linked_vec_len,
    collector::{
        IndexedParallelCollector, IntoParallelCollectorBase, ParallelCollector,
        ParallelCollectorBase, ParallelCollectorByMut, assert_par_collector, plumbing,
    },
    slice::with_in_place_write,
};

///
#[derive(Debug, Clone)]
pub struct IntoParCollector<T>(Vec<T>);

///
#[derive(Debug)]
pub struct ParCollectorMut<'a, T>(&'a mut Vec<T>);

impl<T> IntoParallelCollectorBase for Vec<T>
where
    T: Send,
{
    type Output = Self;

    type IntoParCollector = IntoParCollector<T>;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        assert_par_collector::<_, T>(IntoParCollector(self))
    }
}

impl<'a, T> IntoParallelCollectorBase for &'a mut Vec<T>
where
    T: Send,
{
    type Output = Self;

    type IntoParCollector = ParCollectorMut<'a, T>;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        assert_par_collector::<_, T>(ParCollectorMut(self))
    }
}

impl<T> ParallelCollectorBase for IntoParCollector<T> {
    type Output = Vec<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }
}

impl<T> IndexedParallelCollector<T> for IntoParCollector<T>
where
    T: Send,
{
    fn with_consumer<F>(&mut self, len: usize, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::ConsumerFnOnce<T>,
    {
        self.0.par_collector_mut().with_consumer(len, f)
    }
}

impl<T> ParallelCollector<T> for IntoParCollector<T>
where
    T: Send,
{
    fn with_unindexed_consumer<F>(&mut self, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::UnindexedConsumerFnOnce<T>,
    {
        self.0.par_collector_mut().with_unindexed_consumer(f)
    }
}

impl<'a, T> ParallelCollectorBase for ParCollectorMut<'a, T> {
    type Output = &'a mut Vec<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }
}

impl<'a, T> IndexedParallelCollector<T> for ParCollectorMut<'a, T>
where
    T: Send,
{
    fn with_consumer<F>(&mut self, len: usize, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::ConsumerFnOnce<T>,
    {
        self.0.reserve(len);
        let ret = unsafe { with_in_place_write(self.0.as_mut_ptr_range().end, len, f) };
        unsafe {
            // SAFETY: The region we reserved has been fully written
            // (or else it would have already panicked)
            self.0.set_len(self.0.len() + len);
        }

        (ret, ControlFlow::Continue(()))
    }
}

impl<'a, T> ParallelCollector<T> for ParCollectorMut<'a, T>
where
    T: Send,
{
    fn with_unindexed_consumer<F>(&mut self, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::UnindexedConsumerFnOnce<T>,
    {
        let (ret, chunks, len) = with_linked_vec_len(f);

        self.0.reserve(len);
        for mut chunk in chunks {
            self.0.append(&mut chunk);
        }

        (ret, ControlFlow::Continue(()))
    }
}

impl<T> Default for IntoParCollector<T>
where
    T: Send,
{
    #[inline]
    fn default() -> Self {
        Vec::default().into_par_collector()
    }
}
