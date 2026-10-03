use std::{
    ops::ControlFlow,
    sync::atomic::{AtomicBool, Ordering},
};

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

impl<T> ParIsSorted<T>
where
    T: PartialOrd + Send,
{
    /// Creates a new instance of this parallel collector.
    #[inline]
    pub fn new() -> Self {
        assert_unindexed_par_collector::<_, T>(Self {
            prev: None,
            sorted: AtomicBool::new(true),
        })
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

impl<'a, T> DefineSerial<'a> for ParIsSorted<T>
where
    T: PartialOrd + Send,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<'a, T>>;
}

impl<'a, T> DefineUnindexedSerial<'a> for ParIsSorted<T>
where
    T: PartialOrd + Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial<'a, T>>;
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
            consumer::unindexed(self.prev.take(), &self.sorted),
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
        unique::take_uniquify((consumer::unindexed(self.prev.take(), &self.sorted), drop))
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
            consumer::unindexed(self.prev.take(), &self.sorted),
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
        unique_unindexed::take_uniquify((consumer::unindexed(self.prev.take(), &self.sorted), drop))
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

    pub fn unindexed<T>(
        prev: Option<T>,
        sorted: &AtomicBool,
    ) -> OpaqueUnindexedConsumer!(Serial<'_, T>)
    where
        T: PartialOrd + Send,
    {
        BasicUnindexedConsumer {
            state: (sorted, Cell::new(prev)),
            split_f: |(sorted, prev)| (sorted, prev.take().into()),
            combiner_f: |(sorted, _)| {
                |left, right| match (left, right) {
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
                    ) if right_first < *left_last => sorted.store(false, Ordering::Relaxed),

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
            ma_f: |(sorted, _), request| super::max_afford(sorted, request),
            collector_f: |(sorted, prev)| Serial {
                sorted,
                state: if let Some(prev) = prev.into_inner() {
                    FirstLast::Some {
                        first: prev,
                        last: None,
                    }
                } else {
                    FirstLast::None
                },
            },
        }
    }

    pub struct Serial<'a, T>
    where
        T: PartialOrd + Send,
    {
        sorted: &'a AtomicBool,
        state: FirstLast<T>,
    }

    pub enum FirstLast<T>
    where
        T: PartialOrd + Send,
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
        T: PartialOrd + Send,
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

    impl<T> CollectorBase for Serial<'_, T>
    where
        T: PartialOrd + Send,
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

    impl<T> Collector<T> for Serial<'_, T>
    where
        T: PartialOrd + Send,
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
            if item < *prev {
                self.sorted.store(false, Ordering::Relaxed);
                ControlFlow::Break(())
            } else {
                super::control_flow(self.sorted)?;
                *last = Some(item);
                ControlFlow::Continue(())
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

                if item < *first {
                    self.sorted.store(false, Ordering::Relaxed);
                    return ControlFlow::Break(());
                } else {
                    super::control_flow(self.sorted)?;
                    last.insert(item)
                }
            };

            items.try_for_each(|item| {
                if item < *prev {
                    self.sorted.store(false, Ordering::Relaxed);
                    ControlFlow::Break(())
                } else {
                    super::control_flow(self.sorted)?;
                    *prev = item;
                    ControlFlow::Continue(())
                }
            })
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
