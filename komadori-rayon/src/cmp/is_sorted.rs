use std::{
    fmt::Debug,
    ops::ControlFlow,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    collector::{
        Map, MapBase, ParallelCollectorBase, UnindexedParallelCollectorBase,
        assert_unindexed_par_collector,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, SerialOf, SerialOutputOf,
            UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    helpers::{unique, unique_unindexed},
    ops::BasicParClosure,
};

/// A parallel collector that determines whether items are collected in sorted order.
///
/// The [`Output`] remains `true` as long as every item
/// is greater than or equal to the previously item.
/// otherwise the [`Output`] is `false`.
/// If this parallel collector has not collected any items or only collected one item,
/// the [`Output`] is `true`.
///
/// This parallel collector corresponds to [`Iterator::is_sorted()`].
///
/// [`Output`]: ParallelCollectorBase::Output
/// [`Break(())`]: ControlFlow::Break
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParIsSorted};
///
/// let is_sorted = [1, 2, 3]
///     .into_par_iter()
///     .feed_into(ParIsSorted::new());
///
/// assert!(is_sorted);
/// ```
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParIsSorted};
///
/// let is_sorted = [1, 3, 2]
///     .into_par_iter()
///     .feed_into(ParIsSorted::new());
///
/// assert!(!is_sorted);
/// ```
#[derive(Debug)]
pub struct ParIsSorted<T> {
    prev: Option<T>,
    sorted: AtomicBool,
}

/// A parallel collector that determines whether items are collected in sorted order.
///
/// The [`Output`] remains `true` as long as every item
/// and its previously item are sorted according to a given predicate,
/// otherwise the [`Output`] is `false`.
/// If this parallel collector has not collected any items or only collected one item,
/// the [`Output`] is `true`.
///
/// This parallel collector is constructed by [`ParIsSorted::by()`].
///
/// This parallel collector corresponds to [`Iterator::is_sorted_by()`].
///
/// [`Output`]: ParallelCollectorBase::Output
/// [`Break(())`]: ControlFlow::Break
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParIsSorted};
///
/// let is_sorted = [1, 2, 3]
///     .into_par_iter()
///     // Strictly sorted!
///     .feed_into(ParIsSorted::by(|a, b| a < b));
///
/// assert!(is_sorted);
/// ```
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParIsSorted};
///
/// let is_sorted = [1, 2, 2]
///     .into_par_iter()
///     // Strictly sorted!
///     .feed_into(ParIsSorted::by(|a, b| a < b));
///
/// assert!(!is_sorted);
/// ```
pub struct ParIsSortedBy<T, F> {
    prev: Option<T>,
    sorted: AtomicBool,
    compare: F,
}

/// A parallel collector that determines whether items are collected in sorted order.
///
/// The [`Output`] remains `true` as long as the keys of every item
/// and its previously item are sorted, otherwise the [`Output`] is `false`.
/// If this parallel collector has not collected any items or only collected one item,
/// the [`Output`] is `true`.
///
/// This parallel collector is constructed by [`ParIsSorted::by_key()`].
///
/// This parallel collector corresponds to [`Iterator::is_sorted_by_key()`].
///
/// [`Output`]: ParallelCollectorBase::Output
/// [`Break(())`]: ControlFlow::Break
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParIsSorted};
///
/// let is_sorted = [1, -2, 3]
///     .into_par_iter()
///     .feed_into(ParIsSorted::by_key(i32::abs));
///
/// assert!(is_sorted);
/// ```
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParIsSorted};
///
/// let is_sorted = [-2, 1, 3]
///     .into_par_iter()
///     .feed_into(ParIsSorted::by_key(i32::abs));
///
/// assert!(!is_sorted);
/// ```
// We don't use type alias since if so the serial collectors of
// both this struct and the aliased type are the same.
#[derive(Clone)]
pub struct ParIsSortedByKey<K, F>(Map<ParIsSorted<K>, F>);

impl<T> ParIsSorted<T> {
    /// Creates a new instance of this parallel collector.
    #[inline]
    pub const fn new() -> Self
    where
        T: PartialOrd + Send,
    {
        assert_unindexed_par_collector::<_, T>(Self {
            prev: None,
            sorted: AtomicBool::new(true),
        })
    }

    /// Creates a new instance of [`ParIsSortedBy`] with a given comparator function.
    #[inline]
    pub const fn by<F>(compare: F) -> ParIsSortedBy<T, F>
    where
        T: Send,
        F: Fn(&T, &T) -> bool + Sync,
    {
        assert_unindexed_par_collector::<_, T>(ParIsSortedBy {
            prev: None,
            sorted: AtomicBool::new(true),
            compare,
        })
    }

    /// Creates a new instance of [`ParIsSortedByKey`]
    /// with a given key extraction function.
    #[inline]
    pub const fn by_key<K, F>(f: F) -> ParIsSortedByKey<K, F>
    where
        F: Fn(T) -> K + Sync,
        K: PartialOrd + Send,
    {
        let collector = ParIsSorted {
            prev: None,
            sorted: AtomicBool::new(true),
        };
        let collector = MapBase::new_base(collector, BasicParClosure::new(f));
        assert_unindexed_par_collector::<_, T>(ParIsSortedByKey(collector))
    }
}

impl<T> Clone for ParIsSorted<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            prev: self.prev.clone(),
            sorted: self.sorted.load(Ordering::Relaxed).into(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.prev.clone_from(&source.prev);
        *self.sorted.get_mut() = source.sorted.load(Ordering::Relaxed);
    }
}

impl<T> Default for ParIsSorted<T>
where
    T: PartialOrd + Send,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, F> Debug for ParIsSortedBy<T, F>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParIsSortedBy")
            .field("prev", &self.prev)
            .field("sorted", &self.sorted)
            .field("pred", &std::any::type_name::<F>())
            .finish()
    }
}

impl<T, F> Clone for ParIsSortedBy<T, F>
where
    T: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            prev: self.prev.clone(),
            sorted: self.sorted.load(Ordering::Relaxed).into(),
            compare: self.compare.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.prev.clone_from(&source.prev);
        *self.sorted.get_mut() = source.sorted.load(Ordering::Relaxed);
        self.compare.clone_from(&source.compare);
    }
}

impl<K, F> Debug for ParIsSortedByKey<K, F>
where
    K: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ParIsSortedByKey").field(&self.0).finish()
    }
}

impl<'a, T> DefineSerial<'a> for ParIsSorted<T>
where
    T: PartialOrd + Send,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<'a, T, consumer::PartialOrdPredicate>>;
}

impl<'a, T, F> DefineSerial<'a> for ParIsSortedBy<T, F>
where
    T: Send,
    F: Fn(&T, &T) -> bool + Sync,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<'a, T, &'a F>>;
}

impl<'a, K, F> DefineSerial<'a> for ParIsSortedByKey<K, F>
where
    K: PartialOrd + Send,
    F: Sync,
{
    type Serial = unique::Serial<'a, Self, SerialOf<'a, Map<ParIsSorted<K>, F>>>;
}

impl<'a, T> DefineUnindexedSerial<'a> for ParIsSorted<T>
where
    T: PartialOrd + Send,
{
    type UnindexedSerial =
        unique_unindexed::Serial<'a, Self, consumer::Serial<'a, T, consumer::PartialOrdPredicate>>;
}

impl<'a, T, F> DefineUnindexedSerial<'a> for ParIsSortedBy<T, F>
where
    T: Send,
    F: Fn(&T, &T) -> bool + Sync,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial<'a, T, &'a F>>;
}

impl<'a, K, F> DefineUnindexedSerial<'a> for ParIsSortedByKey<K, F>
where
    K: PartialOrd + Send,
    F: Sync,
{
    type UnindexedSerial =
        unique_unindexed::Serial<'a, Self, UnindexedSerialOf<'a, Map<ParIsSorted<K>, F>>>;
}

impl<T> ParallelCollectorBase for ParIsSorted<T>
where
    T: PartialOrd + Send,
{
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        self.sorted.into_inner()
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        max_afford(&self.sorted, request)
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((
            consumer::unindexed(
                self.prev.take(),
                &self.sorted,
                consumer::PartialOrdPredicate,
            ),
            |output| {
                self.prev = output.into_last();
                control_flow(&self.sorted)
            },
        ))
    }

    fn take_parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        unique::take_uniquify((
            consumer::unindexed(
                self.prev.take(),
                &self.sorted,
                consumer::PartialOrdPredicate,
            ),
            drop,
        ))
    }
}

impl<T, F> ParallelCollectorBase for ParIsSortedBy<T, F>
where
    T: Send,
    F: Fn(&T, &T) -> bool + Sync,
{
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        self.sorted.into_inner()
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        max_afford(&self.sorted, request)
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((
            consumer::unindexed(self.prev.take(), &self.sorted, &self.compare),
            |output| {
                self.prev = output.into_last();
                control_flow(&self.sorted)
            },
        ))
    }

    fn take_parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        unique::take_uniquify((
            consumer::unindexed(self.prev.take(), &self.sorted, &self.compare),
            drop,
        ))
    }
}

impl<K, F> ParallelCollectorBase for ParIsSortedByKey<K, F>
where
    K: PartialOrd + Send,
    F: Sync,
{
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0.finish()
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.0.max_afford(request)
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify(self.0.parts(len))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        unique::take_uniquify(self.0.take_parts(len))
    }
}

impl<T> UnindexedParallelCollectorBase for ParIsSorted<T>
where
    T: PartialOrd + Send,
{
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify((
            consumer::unindexed(
                self.prev.take(),
                &self.sorted,
                consumer::PartialOrdPredicate,
            ),
            |output| {
                self.prev = output.into_last();
                control_flow(&self.sorted)
            },
        ))
    }

    fn take_unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>),
    ) {
        unique_unindexed::take_uniquify((
            consumer::unindexed(
                self.prev.take(),
                &self.sorted,
                consumer::PartialOrdPredicate,
            ),
            drop,
        ))
    }
}

impl<T, F> UnindexedParallelCollectorBase for ParIsSortedBy<T, F>
where
    T: Send,
    F: Fn(&T, &T) -> bool + Sync,
{
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify((
            consumer::unindexed(self.prev.take(), &self.sorted, &self.compare),
            |output| {
                self.prev = output.into_last();
                control_flow(&self.sorted)
            },
        ))
    }

    fn take_unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>),
    ) {
        unique_unindexed::take_uniquify((
            consumer::unindexed(self.prev.take(), &self.sorted, &self.compare),
            drop,
        ))
    }
}

impl<K, F> UnindexedParallelCollectorBase for ParIsSortedByKey<K, F>
where
    K: PartialOrd + Send,
    F: Sync,
{
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify(self.0.unindexed_parts())
    }

    fn take_unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>),
    ) {
        unique_unindexed::take_uniquify(self.0.take_unindexed_parts())
    }
}

#[inline]
fn max_afford(sorted: &AtomicBool, request: usize) -> usize {
    if sorted.load(Ordering::Relaxed) {
        request
    } else {
        0
    }
}

#[inline]
fn control_flow(sorted: &AtomicBool) -> ControlFlow<()> {
    if sorted.load(Ordering::Relaxed) {
        ControlFlow::Continue(())
    } else {
        ControlFlow::Break(())
    }
}

#[expect(missing_debug_implementations)]
mod consumer {
    use std::{
        cell::Cell,
        ops::ControlFlow,
        sync::atomic::{AtomicBool, Ordering},
    };

    use crate::collector::plumbing::{
        BasicUnindexedConsumer, Collector, CollectorBase, OpaqueUnindexedConsumer,
        finish_boxed_impl,
    };

    pub fn unindexed<T, P>(
        prev: Option<T>,
        sorted: &AtomicBool,
        sorted_pred: P,
    ) -> OpaqueUnindexedConsumer!(Serial<'_, T, P>)
    where
        T: Send,
        P: IsSortedPredicate<T>,
    {
        BasicUnindexedConsumer {
            state: (sorted, Cell::new(prev), sorted_pred),
            split_f: |(sorted, prev, sorted_pred)| {
                (sorted, prev.take().into(), sorted_pred.clone())
            },
            combiner_f: |(sorted, _, sorted_pred)| {
                let sorted = *sorted;
                let sorted_pred = sorted_pred.clone();
                move |left, right| match (left, right) {
                    (_, FirstLast::None) => {}
                    _ if !sorted.load(Ordering::Relaxed) => {}
                    (left @ FirstLast::None, right) => *left = right,

                    (
                        FirstLast::Some {
                            first: left_last,
                            last: None,
                        }
                        | FirstLast::Some {
                            last: Some(left_last),
                            ..
                        },
                        FirstLast::Some {
                            first: right_first, ..
                        },
                    ) if !sorted_pred.is_sorted(left_last, &right_first) => {
                        sorted.store(false, Ordering::Relaxed)
                    }

                    (
                        FirstLast::Some {
                            last: left_last, ..
                        },
                        FirstLast::Some {
                            first: right_last,
                            last: None,
                        }
                        | FirstLast::Some {
                            last: Some(right_last),
                            ..
                        },
                    ) => *left_last = Some(right_last),
                }
            },
            ma_f: |(sorted, _, _), request| super::max_afford(sorted, request),
            collector_f: |(sorted, prev, sorted_pred)| Serial {
                sorted,
                state: if let Some(prev) = prev.into_inner() {
                    FirstLast::Some {
                        first: prev,
                        last: None,
                    }
                } else {
                    FirstLast::None
                },
                sorted_pred,
            },
        }
    }

    pub struct Serial<'a, T, P>
    where
        T: Send,
    {
        sorted: &'a AtomicBool,
        state: FirstLast<T>,
        sorted_pred: P,
    }

    pub enum FirstLast<T>
    where
        T: Send,
    {
        None,
        Some {
            first: T,
            // We keep the last item to compare to the first item of the right split.
            last: Option<T>,
        },
    }

    impl<T> FirstLast<T>
    where
        T: Send,
    {
        #[inline]
        pub(super) fn into_last(self) -> Option<T> {
            match self {
                Self::None => None,
                Self::Some { first, last: None } => Some(first),
                Self::Some { last, .. } => last,
            }
        }
    }

    impl<T, P> CollectorBase for Serial<'_, T, P>
    where
        T: Send,
        P: IsSortedPredicate<T>,
    {
        type Output = FirstLast<T>;

        #[inline]
        fn finish(self) -> Self::Output {
            self.state
        }

        finish_boxed_impl! {}

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            if self.sorted.load(Ordering::Relaxed) {
                request
            } else {
                0
            }
        }
    }

    impl<T, P> Collector<T> for Serial<'_, T, P>
    where
        T: Send,
        P: IsSortedPredicate<T>,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            let FirstLast::Some { first, last } = &mut self.state else {
                self.state = FirstLast::Some {
                    first: item,
                    last: None,
                };
                return super::control_flow(self.sorted);
            };

            let prev = last.as_ref().unwrap_or(first);
            if self.sorted_pred.is_sorted(prev, &item) {
                super::control_flow(self.sorted)?;
                *last = Some(item);
                ControlFlow::Continue(())
            } else {
                self.sorted.store(false, Ordering::Relaxed);
                ControlFlow::Break(())
            }
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            let mut items = items.into_iter();

            let (first, last) = loop {
                match &mut self.state {
                    state @ FirstLast::None => {
                        let Some(item) = items.next() else {
                            return ControlFlow::Continue(());
                        };

                        *state = FirstLast::Some {
                            first: item,
                            last: None,
                        };
                        super::control_flow(self.sorted)?;
                    }
                    FirstLast::Some { first, last } => break (&*first, last),
                };
            };

            let prev = if let Some(last) = last {
                last
            } else {
                let Some(item) = items.next() else {
                    return ControlFlow::Continue(());
                };

                if self.sorted_pred.is_sorted(first, &item) {
                    super::control_flow(self.sorted)?;
                    last.insert(item)
                } else {
                    self.sorted.store(false, Ordering::Relaxed);
                    return ControlFlow::Break(());
                }
            };

            items.try_for_each(|item| {
                if self.sorted_pred.is_sorted(prev, &item) {
                    super::control_flow(self.sorted)?;
                    *prev = item;
                    ControlFlow::Continue(())
                } else {
                    self.sorted.store(false, Ordering::Relaxed);
                    ControlFlow::Break(())
                }
            })
        }
    }

    // We don't use a plain `Fn` because we need to name the implemented type
    // in `type (Unindexed)Serial`, which isn't possible (for now) for closures.
    pub trait IsSortedPredicate<T>: Clone + Send {
        // So that it can be used in if-guard.
        // We don't plan it to be `FnMut` anyway since we
        // pass a shared reference of an `Fn`.
        fn is_sorted(&self, prev: &T, curr: &T) -> bool;
    }

    // For `ParIsSortedBy`.
    impl<F, T> IsSortedPredicate<T> for F
    where
        F: Fn(&T, &T) -> bool + Clone + Send,
    {
        #[inline]
        fn is_sorted(&self, prev: &T, curr: &T) -> bool {
            self(prev, curr)
        }
    }

    // For `ParIsSorted`.
    #[derive(Clone)]
    pub struct PartialOrdPredicate;
    impl<T> IsSortedPredicate<T> for PartialOrdPredicate
    where
        T: PartialOrd,
    {
        #[inline]
        fn is_sorted(&self, prev: &T, curr: &T) -> bool {
            prev <= curr
        }
    }
}

#[cfg(test)]
mod proptests {
    use std::sync::atomic::Ordering;

    use super::ParIsSorted;

    use crate::test_utils::prelude::*;

    par_collector_test!(indexed {
        iter_data: {
            let mut starting_num = prop_opt(0..=1);
            let mut nums = propvec(0..=1, ..=5);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: {
            let mut collector = ParIsSorted::new();
            collector.prev = starting_num;
            collector
        },
        starting_ma_f: |request| request,
        expected_f: |iter, _, request| {
            let mut last = None;
            let is_sorted = starting_num
                .into_iter()
                .chain(iter)
                .inspect(|&num| last = Some(num))
                .is_sorted();

            (
                is_sorted,
                if is_sorted {
                    Continue((last, request))
                } else {
                    Break(())
                },
            )
        },
        output_pred: PartialEq::eq,
        state_pred: |collector, &expected| collector.prev == expected
            && collector.sorted.load(Ordering::Relaxed),
    });

    unindexed_par_collector_test!(unindexed {
        iter_data: {
            let mut starting_num = prop_opt(0..=1);
            let mut nums = propvec(0..=1, ..=5);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: {
            let mut collector = ParIsSorted::new();
            collector.prev = starting_num;
            collector
        },
        starting_ma_f: |request| request,
        expected_f: |iter, _, request| {
            let mut last = None;
            let is_sorted = starting_num
                .into_iter()
                .chain(iter)
                .inspect(|&num| last = Some(num))
                .is_sorted();

            (
                is_sorted,
                if is_sorted {
                    Continue((last, request))
                } else {
                    Break(())
                },
            )
        },
        output_pred: PartialEq::eq,
        state_pred: |collector, &expected| collector.prev == expected
            && collector.sorted.load(Ordering::Relaxed),
    });
}
