use std::{cmp::Ordering, fmt::Debug, ops::ControlFlow};

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, SerialOf, SerialOutputOf,
            UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    helpers::{unique, unique_unindexed},
};

use super::ValueKey;

/// A parallel collector that computes the minimum value among the items it collects.
///
/// Its [`Output`](ParallelCollectorBase::Output) is `None` if it has not collected any items,
/// or `Some` containing the minimum item otherwise.
///
/// This collector corresponds to [`Iterator::min()`].
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParMin};
///
/// let min = [4, 5, 2, 4, 3]
///     .into_par_iter()
///     .feed_into(ParMin::new());
///
/// assert_eq!(min, Some(2));
/// ```
///
/// The output is `None` if no items were collected.
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParMin};
///
/// let min = ([] as [i32; _])
///     .into_par_iter()
///     .feed_into(ParMin::new());
///
/// assert_eq!(min, None);
/// ```
#[derive(Debug, Clone)]
pub struct ParMin<T> {
    min: Option<T>,
}

/// A parallel collector that computes the minimum value among the items it collects
/// according to a comparison function.
///
/// Its [`Output`](ParallelCollectorBase::Output) is `None` if it has not collected any items,
/// or `Some` containing the minimum item otherwise.
///
/// This parallel collector is constructed by [`ParMin::by()`].
///
/// This parallel collector corresponds to [`Iterator::min_by()`].
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParMin};
///
/// let min = [1.1, -2.3, f64::INFINITY, -1E2, -99.0]
///     .into_par_iter()
///     .feed_into(ParMin::by(f64::total_cmp));
///
/// assert_eq!(min, Some(-1E2));
/// ```
///
/// The output is `None` if no items were collected.
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParMin};
///
/// let min = []
///     .into_par_iter()
///     .feed_into(ParMin::by(f64::total_cmp));
///
/// assert_eq!(min, None);
/// ```
#[derive(Clone)]
pub struct ParMinBy<T, F> {
    min: Option<T>,
    f: F,
}

/// A collector that computes the item among the items it collects
/// that gives the minimum value from a function.
///
/// Its [`Output`](ParallelCollectorBase::Output) is `None` if it has not collected any items,
/// or `Some` containing the minimum item otherwise.
///
/// This parallel collector is constructed by [`ParMin::by_key()`].
///
/// This parallel collector corresponds to [`Iterator::min_by_key()`].
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParMin};
///
/// let min = ["noble", "and", "singer"]
///     .into_par_iter()
///     .feed_into(ParMin::by_key(|s: &&str| s.len()));
///
/// assert_eq!(min, Some("and"));
/// ```
///
/// The output is `None` if no items were collected.
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParMin};
///
/// let min = ([] as [&str; _])
///     .into_par_iter()
///     .feed_into(ParMin::by_key(|s: &&str| s.len()));
///
/// assert_eq!(min, None);
/// ```
#[derive(Clone)]
pub struct ParMinByKey<T, K, F> {
    min: Option<ValueKey<T, K>>,
    f: F,
}

impl<T> ParMin<T> {
    /// Creates a new instance of this parallel collector.
    #[inline]
    pub const fn new() -> Self
    where
        T: Ord + Send,
    {
        assert_unindexed_par_collector::<_, T>(Self { min: None })
    }

    /// Creates a new instance of [`ParMinBy`] with a given comparison function.
    #[inline]
    pub const fn by<F>(f: F) -> ParMinBy<T, F>
    where
        T: Send,
        F: Fn(&T, &T) -> Ordering + Sync,
    {
        assert_unindexed_par_collector::<_, T>(ParMinBy { min: None, f })
    }

    /// Creates a new instance of [`ParMinByKey`] with a given key-extraction function.
    #[inline]
    pub const fn by_key<K, F>(f: F) -> ParMinByKey<T, K, F>
    where
        T: Send,
        K: Ord + Send,
        F: Fn(&T) -> K + Sync,
    {
        assert_unindexed_par_collector::<_, T>(ParMinByKey { min: None, f })
    }
}

impl<T> Default for ParMin<T>
where
    T: Ord + Send,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug, F> Debug for ParMinBy<T, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ParMinBy")
            .field("min", &self.min)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

impl<T: Debug, K: Debug, F> Debug for ParMinByKey<T, K, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParMinByKey")
            .field("min", &self.min)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

impl<'this, T> DefineSerial<'this> for ParMin<T>
where
    T: Ord + Send,
{
    type Serial = unique::Serial<'this, Self, consumer::Serial<T>>;
}

impl<'a, T, F> DefineSerial<'a> for ParMinBy<T, F>
where
    T: Send,
    F: Fn(&T, &T) -> Ordering + Sync,
{
    type Serial = unique::Serial<'a, Self, consumer::SerialBy<T, &'a F>>;
}

impl<'a, T, K, F> DefineSerial<'a> for ParMinByKey<T, K, F>
where
    T: Send,
    K: Ord + Send,
    F: Fn(&T) -> K + Sync,
{
    type Serial = unique::Serial<'a, Self, consumer::SerialByKey<T, K, &'a F>>;
}

impl<'this, T> DefineUnindexedSerial<'this> for ParMin<T>
where
    T: Ord + Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, consumer::Serial<T>>;
}

impl<'a, T, F> DefineUnindexedSerial<'a> for ParMinBy<T, F>
where
    T: Send,
    F: Fn(&T, &T) -> Ordering + Sync,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::SerialBy<T, &'a F>>;
}

impl<'a, T, K, F> DefineUnindexedSerial<'a> for ParMinByKey<T, K, F>
where
    T: Send,
    K: Ord + Send,
    F: Fn(&T) -> K + Sync,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::SerialByKey<T, K, &'a F>>;
}

impl<T> ParallelCollectorBase for ParMin<T>
where
    T: Ord + Send,
{
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.min
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((consumer::unindexed(), |output| {
            combine(&mut self.min, output);
            ControlFlow::Continue(())
        }))
    }
}

impl<T, F> ParallelCollectorBase for ParMinBy<T, F>
where
    T: Send,
    F: Fn(&T, &T) -> Ordering + Sync,
{
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.min
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((consumer::unindexed_by(&self.f), |output| {
            combine_by(&mut self.min, output, &self.f);
            ControlFlow::Continue(())
        }))
    }
}

impl<T, K, F> ParallelCollectorBase for ParMinByKey<T, K, F>
where
    T: Send,
    K: Ord + Send,
    F: Fn(&T) -> K + Sync,
{
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        Some(self.min?.into_value())
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((consumer::unindexed_by_key(&self.f), |output| {
            combine(&mut self.min, output);
            ControlFlow::Continue(())
        }))
    }
}

impl<T> UnindexedParallelCollectorBase for ParMin<T>
where
    T: Ord + Send,
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
        unique_unindexed::uniquify((consumer::unindexed(), |output| {
            combine(&mut self.min, output);
            ControlFlow::Continue(())
        }))
    }
}

impl<T, F> UnindexedParallelCollectorBase for ParMinBy<T, F>
where
    T: Send,
    F: Fn(&T, &T) -> Ordering + Sync,
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
        unique_unindexed::uniquify((consumer::unindexed_by(&self.f), |output| {
            combine_by(&mut self.min, output, &self.f);
            ControlFlow::Continue(())
        }))
    }
}

impl<T, K, F> UnindexedParallelCollectorBase for ParMinByKey<T, K, F>
where
    T: Send,
    K: Ord + Send,
    F: Fn(&T) -> K + Sync,
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
        unique_unindexed::uniquify((consumer::unindexed_by_key(&self.f), |output| {
            combine(&mut self.min, output);
            ControlFlow::Continue(())
        }))
    }
}

#[inline]
fn combine<T: Ord>(left: &mut Option<T>, right: Option<T>) {
    combine_by(left, right, Ord::cmp);
}

#[inline]
fn combine_by<T>(left: &mut Option<T>, right: Option<T>, f: impl FnOnce(&T, &T) -> Ordering) {
    crate::iter::combine_opt(left, right, |left, right| {
        if f(&right, left).is_lt() {
            *left = right;
        }
    });
}

#[expect(missing_debug_implementations)]
mod consumer {
    use std::{cmp::Ordering, ops::ControlFlow};

    use crate::{
        collector::plumbing::{
            BasicUnindexedConsumer, Collector, CollectorBase, OpaqueUnindexedConsumer,
            finish_boxed_impl,
        },
        ops::CallMut,
    };

    use super::ValueKey;

    pub fn unindexed<T>() -> OpaqueUnindexedConsumer!(Serial<T>)
    where
        T: Ord + Send,
    {
        BasicUnindexedConsumer {
            state: (),
            split_f: |_| {},
            combiner_f: |_| super::combine,
            ma_f: |_, request| request,
            collector_f: |_| Serial::new(),
        }
    }

    pub fn unindexed_by<T, F>(f: &F) -> OpaqueUnindexedConsumer!(SerialBy<T, &F>)
    where
        T: Send,
        F: Fn(&T, &T) -> Ordering + Sync,
    {
        BasicUnindexedConsumer {
            state: f,
            split_f: Clone::clone,
            combiner_f: |&f| move |left, right| super::combine_by(left, right, f),
            ma_f: |_, request| request,
            collector_f: |f| Serial::by(f),
        }
    }

    pub fn unindexed_by_key<T, K, F>(f: &F) -> OpaqueUnindexedConsumer!(SerialByKey<T, K, &F>)
    where
        T: Send,
        K: Ord + Send,
        F: Fn(&T) -> K + Sync,
    {
        BasicUnindexedConsumer {
            state: f,
            split_f: Clone::clone,
            combiner_f: |_| super::combine,
            ma_f: |_, request| request,
            collector_f: |f| SerialByKey {
                minner: Serial::new(),
                f,
            },
        }
    }

    pub type Serial<T> = komadori::cmp::Min<T>;

    pub type SerialBy<T, F> = komadori::cmp::MinBy<T, F>;

    pub struct SerialByKey<T, K, F> {
        minner: Serial<ValueKey<T, K>>,
        f: F,
    }

    impl<T, K, F> CollectorBase for SerialByKey<T, K, F> {
        type Output = Option<ValueKey<T, K>>;

        #[inline]
        fn finish(self) -> Self::Output {
            self.minner.finish()
        }

        finish_boxed_impl! {}
    }

    impl<T, K, F> Collector<T> for SerialByKey<T, K, F>
    where
        K: Ord,
        // We prepare for the possibility of `by_key_with`,
        // which's possible since the closure isn't used in `combine`.
        F: for<'a> CallMut<(&'a T,), Output = K>,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            self.minner
                .collect(ValueKey::new(item, |item| self.f.call_mut((item,))))
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            self.minner.collect_many(
                items
                    .into_iter()
                    .map(|item| ValueKey::new(item, |item| self.f.call_mut((item,)))),
            )
        }

        #[inline]
        fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
            self.minner.collect_then_finish(
                items
                    .into_iter()
                    .map(move |item| ValueKey::new(item, |item| self.f.call_mut((item,)))),
            )
        }
    }
}
