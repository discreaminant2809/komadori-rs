use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::assert_unindexed_par_collector_base;

use super::{
    Filter, FilterWith, NestLocal, NestLocalWith, ParallelCollectorBase, TakeAnyWhile,
    assert_unindexed_par_collector,
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

    /// Same as [`filter()`](Self::filter), but with a state that will either be cloned
    /// or created from a factory (or both) to each serial execution.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    /// use komadori::prelude::*;
    /// use std::sync::mpsc::channel;
    ///
    /// let (sender, receiver) = channel();
    ///
    /// let bigs = [1_u32, 2_000_000_000, 300, 4_000_000]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .filter_with(
    ///                 sender, String::new,
    ///                 |sender, buf, &num| {
    ///                     // I know, this is not an efficient way to
    ///                     // count the number of digits.
    ///                     // This is just an example.
    ///                     buf.clear();
    ///                     use std::fmt::Write;
    ///                     write!(buf, "{num}");
    ///
    ///                     if buf.len() >= 7 {
    ///                         true
    ///                     } else {
    ///                         sender.send(num).unwrap();
    ///                         false
    ///                     }
    ///                 },
    ///             ),
    ///     );
    ///
    /// let mut smalls = receiver.iter().feed_into(vec![]);
    /// smalls.sort_unstable();
    ///
    /// assert_eq!(bigs, [2_000_000_000, 4_000_000]);
    /// assert_eq!(smalls, [1, 300]);
    /// ```
    #[inline]
    fn filter_with<L1, FL2, L2, P, T>(
        self,
        local1: L1,
        local2_f: FL2,
        pred: P,
    ) -> FilterWith<Self, L1, FL2, P>
    where
        Self: UnindexedParallelCollector<T> + Sized,
        L1: Clone + Send,
        FL2: Fn() -> L2 + Sync,
        P: Fn(&mut L1, &mut L2, &T) -> bool + Sync,
    {
        assert_unindexed_par_collector::<_, T>(FilterWith::new(self, local1, local2_f, pred))
    }

    /// Creates a parallel collector that accumulates items until it encounters
    /// an item that makes a given predicate `false` at *any* time.
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

    /// Creates a parallel collector that collects all the outputs
    /// from local collectors cloned to each serial reduction.
    ///
    /// `nest_local()` is usually used after [`ParReduce`](crate::iter::ParReduce).
    ///
    /// This adapter collects `T` if `C: IntoCollector<T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::{prelude::*, iter::ParReduce};
    ///
    /// let nums = (1..=5)
    ///     .into_par_iter()
    ///     .feed_into(
    ///         ParReduce::new(|v1, mut v2: Vec<_>| v1.append(&mut v2))
    ///             .nest_local(vec![])
    ///     );
    ///
    /// assert_eq!(nums, Some(vec![1, 2, 3, 4, 5]));
    /// ```
    #[inline]
    fn nest_local<C>(self, local: C) -> NestLocal<Self, C::IntoCollector>
    where
        Self: UnindexedParallelCollector<C::Output> + Sized,
        C: IntoCollectorBase<IntoCollector: Clone + Send>,
    {
        assert_unindexed_par_collector_base(NestLocal::new(self, local.into_collector()))
    }

    /// Creates a parallel collector that collects all the outputs
    /// from local collectors created from a function to each serial reduction.
    ///
    /// `nest_local_with()` is usually used after [`ParReduce`](crate::iter::ParReduce).
    ///
    /// This adapter collects `T` if `C: IntoCollector<T>`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori::prelude::*;
    /// use komadori_rayon::{prelude::*, iter::ParReduce};
    /// use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    ///
    /// let nums = (1..=5)
    ///     .into_par_iter()
    ///     .feed_into(
    ///         // A rough recreation of `take_any_while()`!
    ///         ParReduce::new(|v1, mut v2: Vec<_>| v1.append(&mut v2))
    ///             .nest_local_with(Arc::new(AtomicBool::new(false)), |stopped| {
    ///                 vec![]
    ///                     .into_collector()
    ///                     .take_while(move |&num| {
    ///                         if stopped.load(Ordering::Relaxed) {
    ///                             false
    ///                         } else if num <= 3 {
    ///                             true
    ///                         } else {
    ///                             stopped.store(true, Ordering::Relaxed);
    ///                             false
    ///                         }
    ///                     })
    ///             })
    ///             .map_output(Option::unwrap_or_default)
    ///     );
    ///
    /// // Honestly we can't guarantee anything other than
    /// // every number must be less than or equal to 3
    /// for num in nums {
    ///     assert!(num <= 3, "{num} is greater than 3");
    /// }
    /// ```
    #[inline]
    fn nest_local_with<L, F, C>(self, local: L, inner_f: F) -> NestLocalWith<Self, L, F>
    where
        Self: UnindexedParallelCollector<C::Output> + Sized,
        L: Clone + Send,
        F: Fn(L) -> C + Sync,
        C: IntoCollectorBase,
    {
        assert_unindexed_par_collector_base(NestLocalWith::new(self, local, inner_f))
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
    C: UnindexedParallelCollectorBase<Serial: Collector<T>, UnindexedSerial: Collector<T>> + ?Sized
{
}
