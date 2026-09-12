use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        assert_unindexed_par_collector_base,
        plumbing::{UnindexedSerialOf, UnindexedSerialOutputOf},
    },
    ops::{ChangeOutputType, Try},
};

use super::{
    Filter, FilterMap, FilterMapWith, FilterWith, FoldLocal, NestLocal, NestLocalWith,
    ParallelCollectorBase, TakeAnyWhile, TryFoldLocal, TryingOptions, TryingResults, UnindexedOnly,
    assert_unindexed_par_collector,
    plumbing::{DefineUnindexedSerial, UnindexedConsumer},
};

/// An unindexed parallel collector.
pub trait UnindexedParallelCollectorBase:
    ParallelCollectorBase + for<'this> DefineUnindexedSerial<'this>
{
    /// Prepares a space to accept *any* amount of items landing on anywhere,
    /// and returns "parts" needed to drive this parallel collector.
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
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
    fn take_unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>),
    ) {
        let (consumer, commit) = self.unindexed_parts();
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
    /// This adapter collects `T`.
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

    /// Same as [`filter()`](Self::filter), but with a shared state and a
    /// clonable "consumer." Each leaf reduction receives a shared reference
    /// to the shared state and a predicate created by the consumer
    /// specifically for this leaf.
    ///
    /// This adapter collects `T`.
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
    ///             .filter_with((), move |_| {
    ///                 let mut buf = String::new();
    ///                 move |_, &num| {
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
    ///                 }
    ///             }),
    ///     );
    ///
    /// let mut smalls = receiver.iter().feed_into(vec![]);
    /// smalls.sort_unstable();
    ///
    /// assert_eq!(bigs, [2_000_000_000, 4_000_000]);
    /// assert_eq!(smalls, [1, 300]);
    /// ```
    #[inline]
    fn filter_with<S, FP, P, T>(self, shared_state: S, consumer: FP) -> FilterWith<Self, S, FP>
    where
        Self: UnindexedParallelCollector<T> + Sized,
        S: Sync,
        FP: FnOnce(&S) -> P + Clone + Send,
        P: FnMut(&S, &T) -> bool,
    {
        assert_unindexed_par_collector::<_, T>(FilterWith::new(self, shared_state, consumer))
    }

    /// A parallel collector that both filters and maps each item before collecting.
    ///
    /// The underlying parallel collector only collects `item`s for which
    /// the given predicate returns [`Some(item)`](Some).
    ///
    /// Note that even if an item is not accumulated, this adapter will still return
    /// [`Continue(())`] as long as the underlying parallel collector does.
    ///
    /// `filter_map()` will **always** use the unindexed path
    /// of the underlying parallel collector,
    /// because the number of items is nondeterministic now.
    ///
    /// This adapter collects `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    ///
    /// let nums = ["1", "-2", "three", "4"]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .filter_map(|s: &str| s.parse::<i32>().ok())
    ///     );
    ///
    /// assert_eq!(nums, [1, -2, 4]);
    /// ```
    ///
    /// [`Continue(())`]: ControlFlow::Continue
    #[inline]
    fn filter_map<P, T, R>(self, pred: P) -> FilterMap<Self, P>
    where
        Self: UnindexedParallelCollector<R> + Sized,
        P: Fn(T) -> Option<R> + Sync,
    {
        assert_unindexed_par_collector::<_, T>(FilterMap::new(self, pred))
    }

    /// Same as [`filter_map()`](Self::filter_map), but with a shared state and a
    /// clonable "consumer." Each leaf reduction receives a shared reference
    /// to the shared state and a predicate created by the consumer
    /// specifically for this leaf.
    ///
    /// This adapter collects `T`.
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
    /// let nums = ["1", "-2", "three", "4"]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .filter_map_with((), move |_| move |_, s: &str| {
    ///                 match s.parse::<i32>() {
    ///                     Ok(num) => Some(num),
    ///                     Err(_) => {
    ///                         sender.send(s);
    ///                         None
    ///                     }
    ///                 }
    ///             })
    ///     );
    ///
    /// let mut nans = receiver.iter().feed_into(vec![]);
    ///
    /// assert_eq!(nums, [1, -2, 4]);
    /// assert_eq!(nans, ["three"]);
    /// ```
    #[inline]
    fn filter_map_with<S, FP, P, T, R>(
        self,
        shared_state: S,
        consumer: FP,
    ) -> FilterMapWith<Self, S, FP>
    where
        Self: UnindexedParallelCollector<R> + Sized,
        S: Sync,
        FP: FnOnce(&S) -> P + Clone + Send,
        P: FnMut(&S, T) -> Option<R>,
    {
        assert_unindexed_par_collector::<_, T>(FilterMapWith::new(self, shared_state, consumer))
    }

    /// Creates a parallel collector that accumulates items until it encounters
    /// an item that makes a given predicate `false` at *any* time.
    ///
    /// `take_any_while()` will **always** use the unindexed path
    /// of the underlying parallel collector,
    /// because the number of items is nondeterministic now.
    ///
    /// This adapter collects `T`.
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
    /// The underlying parallel collector will receive an output of the cloned local collector
    /// after each local reduction ends.
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
    /// The underlying parallel collector will receive an output of the created local collector
    /// after each local reduction ends.
    ///
    /// `nest_local_with()` is usually used after [`ParReduce`](crate::iter::ParReduce).
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
    ///         ParReduce::new(|v1, v2: Vec<_>| v1.extend(v2))
    ///             .nest_local_with((), |_| vec![])
    ///     );
    ///
    /// assert_eq!(nums, Some(vec![1, 2, 3, 4, 5]));
    /// ```
    #[inline]
    fn nest_local_with<S, F, C>(self, shared_state: S, consumer: F) -> NestLocalWith<Self, S, F>
    where
        Self: UnindexedParallelCollector<C::Output> + Sized,
        S: Sync,
        F: FnOnce(&S) -> C + Clone + Send,
        C: IntoCollectorBase,
    {
        assert_unindexed_par_collector_base(NestLocalWith::new(self, shared_state, consumer))
    }

    /// Creates a parallel collector that uses a closure and local states
    /// to collect items in each local reduction.
    ///
    /// The underlying parallel collector will receive a tuple of both local states
    /// after each local reduction ends.
    ///
    /// `fold_local()` is usually used after [`ParReduce`](crate::iter::ParReduce).
    /// You can also use [`map()`](ParallelCollectorBase::map) between the two
    /// to get rid of the tuple.
    ///
    /// This adapter collects `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::{prelude::*, iter::ParReduce};
    ///
    /// let sentence = ["there", "are", "a", "noble", "and", "a", "singer"]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         ParReduce::new(|piece1, piece2: String| {
    ///             piece1.push(' ');
    ///             *piece1 += &piece2;
    ///         })
    ///         .fold_local((), |_| {
    ///             let mut is_first = true;
    ///             (String::new(), move |_, piece, word| {
    ///                 if !std::mem::replace(&mut is_first, false) {
    ///                     piece.push(' ');
    ///                 }
    ///                 *piece += word;
    ///             })
    ///         })
    ///         .map_output(Option::unwrap_or_default)
    ///     );
    ///
    /// assert_eq!(sentence, "there are a noble and a singer");
    /// ```
    #[inline]
    fn fold_local<S, A, FF, F, T>(self, shared_state: S, consumer: FF) -> FoldLocal<Self, S, FF>
    where
        Self: UnindexedParallelCollector<A> + Sized,
        S: Sync,
        FF: FnOnce(&S) -> (A, F) + Clone + Send,
        F: FnMut(&S, &mut A, T) + Sync,
    {
        assert_unindexed_par_collector::<_, T>(FoldLocal::new(self, shared_state, consumer))
    }

    /// Creates a parallel collector that produces each item from each local reduction
    /// to feed into the underlying parallel collector, which an ability
    /// to stop early.
    ///
    /// Components:
    ///
    /// - `shared_state` (`S`): States that will be shared between worker.
    ///   Must be [`Sync`].
    ///
    /// - `consumer` (`FF`): Things happen inside the splitting process.
    ///   Must implement [`Clone`] and [`Send`].
    ///
    ///   The function will be cloned when being split.
    ///   When a worker decides to perform a reduction, the function will be called.
    ///   If it returns a "break" value, the reduction stops immediately
    ///   and the item of this reduction will be that "break" value.
    ///   Otherwise, an initial state and a fold function will be returned,
    ///   and the reduction progresses similarly to [`TryFold`] from `komadori`
    ///   with the output being the item of this reduction.
    ///
    /// - Fold function (`F`): Created alongside with an initial state from `consumer`.
    ///   No additional trait requirement.
    ///
    ///   Each incoming item will be called with this function to update the state,
    ///   and return a "continue" value to continue folding or "break" value
    ///   to stop early.
    ///
    /// As of now, the permitted types for `A` is [`Option`], [`Result`],
    /// and [`ControlFlow`].
    ///
    /// [`TryFold`]: komadori::iter::TryFold
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
    /// use rayon::prelude::*;
    /// use komadori_rayon::{prelude::*, iter::ParReduce};
    ///
    /// fn collect_vec_while<T: Send>(
    ///     mut pred: impl FnMut(&T) -> bool + Clone + Send,
    /// ) -> impl UnindexedParallelCollector<T, Output = Vec<T>> {
    ///     ParReduce::new(|v1, v2: Vec<_>| v1.extend(v2))
    ///         .map_output(|v| v.unwrap_or(vec![]))
    ///         .map(|res: Result<_, _>| res.unwrap_or_else(|v| v))
    ///         .try_fold_local(AtomicBool::new(false), move |stopped| {
    ///             if stopped.load(Relaxed) {
    ///                 return Err(vec![]);
    ///             }
    ///
    ///             Ok((
    ///                 vec![],
    ///                 move |stopped: &AtomicBool, chunk: &mut Vec<_>, item| {
    ///                     if stopped.load(Relaxed) {
    ///                         Err(std::mem::take(chunk))
    ///                     } else if pred(&item) {
    ///                         chunk.push(item);
    ///                         Ok(())
    ///                     } else {
    ///                         stopped.store(true, Relaxed);
    ///                         Err(std::mem::take(chunk))
    ///                     }
    ///                 },
    ///             ))
    ///         })
    /// }
    ///
    /// let nums = [1, 2, 3, 4, 5]
    ///     .into_par_iter()
    ///     .feed_into(collect_vec_while(|&num| num > 0));
    ///
    /// assert_eq!(nums, [1, 2, 3, 4, 5]);
    /// ```
    #[inline]
    fn try_fold_local<S, A, Acc, FF, F, T>(
        self,
        shared_state: S,
        consumer: FF,
    ) -> TryFoldLocal<Self, S, FF>
    where
        Self: UnindexedParallelCollector<ChangeOutputType<A, Acc>> + Sized,
        S: Sync,
        A: Try<Output = (Acc, F)>,
        FF: FnOnce(&S) -> A + Clone + Send,
        F: FnMut(&S, &mut Acc, T) -> ChangeOutputType<A, ()>,
    {
        assert_unindexed_par_collector::<_, T>(TryFoldLocal::new(self, shared_state, consumer))
    }

    /// Creates a parallel collector that sets the [`Output`] to [`None`] when
    /// a [`None`] item is encountered *anywhere*,
    /// else the underlying collector collects the `item` inside
    /// [`Some(item)`](Some).
    ///
    /// This is analogous to when you collect an iterator of [`Option<T>`]
    /// to an `Option<Collection<T>>`.
    ///
    /// This adapter collects [`Option<T>`] if the
    /// underlying paralllel collector collects `T`.
    ///
    /// [`Output`]: ParallelCollectorBase::Output
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    ///
    /// let res = [Some(1), Some(2), Some(3)]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .trying_options()
    ///     );
    ///
    /// assert_eq!(res, Some(vec![1, 2, 3]));
    /// ```
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    ///
    /// let res = [Some(1), None, Some(3)]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .trying_options()
    ///     );
    ///
    /// assert_eq!(res, None);
    /// ```
    #[inline]
    fn trying_options(self) -> TryingOptions<Self>
    where
        Self: Sized,
    {
        assert_unindexed_par_collector_base(TryingOptions::new(self))
    }

    /// Creates a parallel collector that sets the [`Output`] to [`Err(e)`](Err) when
    /// an [`Err(e)`](Err) item is encountered *anywhere*,
    /// else the underlying collector collects the `item` inside
    /// [`Ok(item)`](Ok).
    ///
    /// This is analogous to when you collect an iterator of [`Result<T, E>`]
    /// to a `Result<Collection<T>, E>`.
    ///
    /// If there are more than one errors encountered, it is unspecified
    /// which one will be kept as an output.
    ///
    /// This adapter collects [`Result<T, E>`] if the
    /// underlying paralllel collector collects `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    ///
    /// let res = [Ok(1), Ok(2), Ok(3)]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .trying_results::<&str>()
    ///     );
    ///
    /// assert_eq!(res, Ok(vec![1, 2, 3]));
    /// ```
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    ///
    /// let res = [Ok(1), Err("can't collect anymore"), Ok(3)]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .trying_results()
    ///     );
    ///
    /// assert_eq!(res, Err("can't collect anymore"));
    /// ```
    ///
    /// [`Output`]: ParallelCollectorBase::Output
    #[inline]
    fn trying_results<E>(self) -> TryingResults<Self, E>
    where
        Self: Sized,
        E: Send,
    {
        assert_unindexed_par_collector_base(TryingResults::new(self))
    }

    /// Creates a parallel collector that restricts to the unindexed path only.
    ///
    /// No matter whichever path (indexed or unindexed) you ask it for,
    /// `unindexed_only()` always uses the unindexed path of the underlying parallel collector.
    /// However, it does **not** alter the path the upstream (which provides items
    /// for the parallel collector) chooses.
    ///
    /// This adapter might be useful if you want to benchmark the unindexed path explicitly
    /// without the code implicitly switching to the indexed path.
    /// This is also useful if you want consistent semantics, such as you want
    /// [`take(n)`](ParallelCollectorBase::take) to always take `n` random items
    /// instead of the first `n` items for the indexed path.
    ///
    /// This adapter collects `T` if the underlying parallel collector collects `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use komadori_rayon::prelude::*;
    /// use std::assert_matches;
    ///
    /// let three_nums = [1, 5, 4, 2, 3]
    ///     .into_par_iter()
    ///     .feed_into(
    ///         vec![]
    ///             .into_par_collector()
    ///             .take(3)
    ///             .unindexed_only()
    ///     );
    ///
    /// // Now we can only assume that there are three numbers
    /// // that come from random positions.
    /// assert_eq!(three_nums.len(), 3);
    /// for num in three_nums {
    ///     assert_matches!(num, 1..=5, "{num} is not in between 1 and 5");
    /// }
    /// ```
    #[inline]
    fn unindexed_only(self) -> UnindexedOnly<Self>
    where
        Self: Sized,
    {
        assert_unindexed_par_collector_base(UnindexedOnly::new(self))
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
