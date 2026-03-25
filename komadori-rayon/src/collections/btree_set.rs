//!

use std::{collections::BTreeSet, ops::ControlFlow};

use komadori::prelude::*;

use crate::collector::{
    IntoParallelCollectorBase, ParallelCollectorBase, UnindexedParallelCollectorBase,
    assert_par_collector,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
};

use super::linked_vec;

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

impl<'this, T> DefineConsumer<'this> for IntoParCollector<T>
where
    T: Send,
{
    type Consumer = in_pla;
}

impl<T> ParallelCollectorBase for IntoParCollector<T>
where
    T: Ord + Send,
{
    type Output = BTreeSet<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        (len, linked_vec::Consumer::new(), move |(chunks, _)| {
            for chunk in chunks {
                self.0.extend(chunk);
            }

            ControlFlow::Continue(())
        })
    }
}

impl<'this, T> DefineUnindexedConsumer<'this> for IntoParCollector<T>
where
    T: Send,
{
    type UnindexedConsumer = linked_vec::Consumer<T, ()>;
}

impl<T> UnindexedParallelCollectorBase for IntoParCollector<T>
where
    T: Ord + Send,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        (linked_vec::Consumer::new(), move |(chunks, _)| {
            for chunk in chunks {
                self.0.extend(chunk);
            }

            ControlFlow::Continue(())
        })
    }
}
