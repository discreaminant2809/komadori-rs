use std::ops::ControlFlow;

use komadori::prelude::*;

use super::{
    Filter, ParallelCollectorBase, TakeAnyWhile, assert_unindexed_par_collector,
    plumbing::{DefineUnindexedSerial, UnindexedConsumer},
};

/// An unindexed parallel collector.
pub trait UnindexedParallelCollectorBase:
    ParallelCollectorBase + for<'this> DefineUnindexedSerial<'this>
{
    /// Prepares a space to accept *any* amount of items landing on anywhere,
    /// and returns "parts" needed to drive this parallel collector.
    #[allow(clippy::type_complexity)]
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    );

    /// Prepares a space to accept *any* amount of items landing on anywhere,
    /// and returns "parts" needed to drive this parallel collector.
    ///
    /// This method effectively "consumes" the collector.
    /// After calling this method, the collector is counted
    /// to have returned [`Break(())`](ControlFlow::Break)
    /// and the only valid method to call is [`finish()`](ParallelCollectorBase::finish).
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
    #[allow(clippy::type_complexity)]
    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output),
    ) {
        let (consumer, commit) = self.parts_unindexed();
        (consumer, |output| {
            let _ = commit(output);
        })
    }

    /// Creates a parallel collector that uses a closure to determine whether
    /// an item should be accumulated.
    ///
    /// The underlying parallel collector only collects items for which
    /// the given predicate returns `true`.
    ///
    /// Note that even if an item is not accumulated, this adapter will still return
    /// [`Continue(())`] as long as the underlying parallel collector does.
    /// If you want the collector to stop after the first `false`,
    /// consider using [`take_any_while()`](Self::take_any_while) instead.
    ///
    /// `filter()` will **always** use the unindexed path
    /// of the underlying parallel collector,
    /// because the number of items is nondeterministic now.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    ///
    /// let evens = [1, 2, 4, 5]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .filter(|&x| x % 2 == 0)
    ///     );
    ///
    /// assert_eq!(evens, [2, 4]);
    /// ```
    ///
    /// [`Continue(())`]: ControlFlow::Continue
    #[inline]
    fn filter<P, T>(self, pred: P) -> Filter<Self, P>
    where
        Self: UnindexedParallelCollector<T> + Sized,
        P: Fn(&T) -> bool + Sync,
    {
        assert_unindexed_par_collector::<_, T>(Filter::new(self, pred))
    }

    /// Creates a parallel collector that accumulates items until it encounters
    /// an items that makess a given predicate `false` at *any* time.
    ///
    /// `take_any_while()` will **always** use the unindexed path
    /// of the underlying parallel collector,
    /// because the number of items is nondeterministic now.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    ///
    /// let result: Vec<_> = (0..100)
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .take_any_while(|x| *x < 50)
    ///     );
    ///
    /// assert!(result.len() <= 50);
    /// assert!(result.windows(2).all(|w| w[0] < w[1]));
    /// ```
    #[inline]
    fn take_any_while<P, T>(self, pred: P) -> TakeAnyWhile<Self, P>
    where
        Self: UnindexedParallelCollector<T> + Sized,
        P: Fn(&T) -> bool + Sync,
    {
        assert_unindexed_par_collector::<_, T>(TakeAnyWhile::new(self, pred))
    }
}

/// Defines what item types are collected in an unindexed parallel collector.
///
/// You cannot implement this trait directly. You should instead define the item type
/// of serial collectors produced by consumers of this parallel collector.
pub trait UnindexedParallelCollector<T>:
    UnindexedParallelCollectorBase<Serial: Collector<T>, UnindexedSerial: Collector<T>>
{
}

impl<C, T> UnindexedParallelCollector<T> for C where
    C: UnindexedParallelCollectorBase<Serial: Collector<T>, UnindexedSerial: Collector<T>>
{
}
