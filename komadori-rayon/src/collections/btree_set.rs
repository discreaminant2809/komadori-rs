use std::{collections::BTreeSet, ops::ControlFlow};

use crate::{
    collections::reservable::produce_linked_vec,
    collector::{
        IndexedParallelCollector, IntoParallelCollectorBase, ParallelCollector,
        ParallelCollectorBase, assert_par_collector, plumbing,
    },
};

///
#[derive(Debug, Clone)]
pub struct IntoParCollector<T>(BTreeSet<T>);

impl<T> IntoParallelCollectorBase for BTreeSet<T>
where
    T: Ord + Send,
{
    type Output = Self;

    type IntoParCollector = IntoParCollector<T>;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        assert_par_collector::<_, T>(IntoParCollector(self))
    }
}

impl<T> ParallelCollectorBase for IntoParCollector<T> {
    type Output = BTreeSet<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }
}

impl<T> IndexedParallelCollector<T> for IntoParCollector<T>
where
    T: Ord + Send,
{
    fn with_consumer<F>(&mut self, len: usize, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::ConsumerFnOnce<T>,
    {
        todo!()
    }
}

impl<T> ParallelCollector<T> for IntoParCollector<T>
where
    T: Ord + Send,
{
    fn with_unindexed_consumer<F>(&mut self, f: F) -> (F::Output, ControlFlow<()>)
    where
        F: plumbing::UnindexedConsumerFnOnce<T>,
    {
        let (ret, chunks) = produce_linked_vec(f);
        for chunk in chunks {
            self.0.extend(chunk);
        }

        (ret, ControlFlow::Continue(()))
    }
}
