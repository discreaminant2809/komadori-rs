//!

use std::{collections::BTreeSet, ops::ControlFlow};

use crate::collector::{
    IntoParallelCollectorBase, ParallelCollectorBase, assert_par_collector, plumbing,
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

impl<'this, T> plumbing::DefineConsumer<'this> for IntoParCollector<T>
where
    T: Send,
{
    type Consumer = linked_vec::Consumer<T, ()>;
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
            <Self as plumbing::DefineConsumer<'a>>::Consumer,
            impl FnOnce(
                <<Self as plumbing::DefineConsumer<'a>>::Consumer as komadori::prelude::IntoCollectorBase>::Output,
            ) -> ControlFlow<()>,
    ){
        (len, linked_vec::Consumer::new(), move |(chunks, _)| {
            for chunk in chunks {
                self.0.extend(chunk);
            }

            ControlFlow::Continue(())
        })
    }
}
