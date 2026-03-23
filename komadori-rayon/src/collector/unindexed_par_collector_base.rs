use std::{marker::PhantomData, ops::ControlFlow};

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

    /// For the reason why [`PhantomData`] is needed in the closure,
    /// see [here](https://doc.rust-lang.org/error_codes/E0582.html).
    fn with_unindexed_consumer<R>(
        self,
        f: impl for<'a> FnOnce(
            <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
            PhantomData<&'a ()>,
        ) -> (
            R,
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ),
    ) -> (R, Self::Output) {
        let mut this = self;
        let (consumer, committer) = this.parts_unindexed();
        let (ret, output) = f(consumer, PhantomData);
        committer(output);
        (ret, this.finish())
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
