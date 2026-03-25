use std::ops::ControlFlow;

use komadori::prelude::*;

use super::{
    ParallelCollectorBase,
    plumbing::{Consumer, DefineUnindexedConsumer, UnindexedConsumer},
};

///
pub trait UnindexedParallelCollectorBase:
    ParallelCollectorBase + for<'this> DefineUnindexedConsumer<'this>
{
    ///
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    );

    ///
    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ),
    ) {
        let (consumer, commit) = self.parts_unindexed();
        (consumer, |output| {
            let _ = commit(output);
        })
    }
}

///
pub trait UnindexedParallelCollector<T>:
    UnindexedParallelCollectorBase<Consumer: Consumer<T>, UnindexedConsumer: UnindexedConsumer<T>>
{
}

impl<C, T> UnindexedParallelCollector<T> for C where
    C: UnindexedParallelCollectorBase<
            Consumer: Consumer<T>,
            UnindexedConsumer: UnindexedConsumer<T>,
        >
{
}
