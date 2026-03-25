use std::ops::ControlFlow;

use komadori::prelude::*;

use super::{
    ParallelCollectorBase,
    plumbing::{Consumer, DefineUnindexedConsumer, UnindexedConsumer},
};

/// An unindexed parallel collector.
pub trait UnindexedParallelCollectorBase:
    ParallelCollectorBase + for<'this> DefineUnindexedConsumer<'this>
{
    /// Prepares a space to accept *any* amount of items landing on anywhere,
    /// and returns "parts" needed to drive this parallel collector.
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    );

    /// Prepares a space to accept *any* amount of items landing on anywhere,
    /// and returns "parts" needed to drive this parallel collector.
    ///
    /// This method effectively "consumes" the collector.
    /// After calling this method, the collector is counted
    /// to have returned [`Break(())`](ControlFlow::Break)
    /// and the only valid method to call is [`finish()`](Self::finish).
    /// The behavior is unspecified if you call other methods than that method,
    /// including panicking or incorrect results.
    /// You can leverage it by "consuming" some states instead of cloning them
    /// for more efficiency.
    ///
    /// Most parallel collectors do not care whether they can
    /// optimize anything by consuming some states
    /// (and hence this method is not required to override),
    /// but if it is the case or you are implementing an adapter,
    /// you should override this method.
    ///
    /// The signature is similar to [`parts_unindexed()`](Self::parts_unindexed),
    /// except the returning function which does not return
    /// a [`ControlFlow`].
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

/// Defines what item types are collected in an unindexed parallel collector.
///
/// You cannot implement this trait directly. You should instead define the item type
/// of serial collectors produced by consumers of this parallel collector.
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
