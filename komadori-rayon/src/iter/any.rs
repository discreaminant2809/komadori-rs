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

/// A parallel collector that stores *any* item
/// out of all items it collected.
///
/// If it collected one item, its [`Output`] is [`Some(item)`](Some)
/// containing whatever item it encountered, otherwise [`None`].
/// The item it chooses to keep is unspecified.
///
/// This parallel collector collects `T`.
///
/// This parallel collector might seem silly, but it enables many patterns,
/// especially when combining with some adapters.
/// See examples for more.
///
/// # Examples
///
/// ```
/// use std::assert_matches;
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParAny};
///
/// let any = [1, 2, 3]
///     .into_par_iter()
///     .feed_into(ParAny::new());
///
/// assert_matches!(any, Some(1 | 2 | 3));
/// ```
///
/// ```
/// use std::assert_matches;
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParAny};
///
/// let any = ([] as [i32; _])
///     .into_par_iter()
///     .feed_into(ParAny::new());
///
/// assert_eq!(any, None);
/// ```
///
/// `ParAny::new().filter(f)` corresponds to
/// [`ParallelIterator::find_any(f)`]:
///
/// ```
/// use std::assert_matches;
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParAny};
///
/// let any = [1, 5, 6, 3, 8]
///     .into_par_iter()
///     .feed_into(ParAny::new().filter(|&num| num % 2 == 0));
///
/// assert_matches!(any, Some(6 | 8));
/// ```
///
/// `ParAny::new().filter_map(f)` corresponds to
/// [`ParallelIterator::find_map_any(f)`]:
///
/// ```
/// use std::assert_matches;
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParAny};
///
/// let any = ["noble", "and", "1", "singer", "2"]
///     .into_par_iter()
///     .feed_into(ParAny::new().filter_map(|s: &str| s.parse().ok()));
///
/// assert_matches!(any, Some(1 | 2));
/// ```
///
/// [`Output`]: ParallelCollectorBase
/// [`ParallelIterator::find_any(f)`]: rayon::iter::ParallelIterator::find_any
/// [`ParallelIterator::find_map_any(f)`]: rayon::iter::ParallelIterator::find_map_any
#[derive(Debug)]
pub struct ParAny<T> {
    value: Option<T>,
    found: AtomicBool,
}

impl<T> ParAny<T>
where
    T: Send,
{
    /// Creates a new instance of this parallel collector.
    #[inline]
    pub const fn new() -> Self {
        assert_unindexed_par_collector::<_, T>(Self {
            value: None,
            found: AtomicBool::new(false),
        })
    }
}

impl<T> Default for ParAny<T>
where
    T: Send,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for ParAny<T>
where
    T: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            found: clone_atomic_bool(&self.found),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.value.clone_from(&source.value);
        self.found = clone_atomic_bool(&self.found);
    }
}

impl<'a, T> DefineSerial<'a> for ParAny<T>
where
    T: Send,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<'a, T>>;
}

impl<'a, T> DefineUnindexedSerial<'a> for ParAny<T>
where
    T: Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial<'a, T>>;
}

impl<T> ParallelCollectorBase for ParAny<T>
where
    T: Send,
{
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.value
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        max_afford(&self.found, request)
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> std::ops::ControlFlow<()>,
    ) {
        unique::uniquify((consumer::Consumer::new(&self.found), |output| {
            combine(&mut self.value, output);
            if self.value.is_some() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }))
    }
}

impl<T> UnindexedParallelCollectorBase for ParAny<T>
where
    T: Send,
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
        unique_unindexed::uniquify((consumer::Consumer::new(&self.found), |output| {
            combine(&mut self.value, output);
            if self.value.is_some() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }))
    }
}

#[inline]
fn clone_atomic_bool(b: &AtomicBool) -> AtomicBool {
    b.load(Ordering::Relaxed).into()
}

#[inline]
fn max_afford(found: &AtomicBool, request: usize) -> usize {
    (request > 0 && !found.load(Ordering::Relaxed)) as _
}

#[inline]
fn combine<T>(left: &mut Option<T>, right: Option<T>) {
    super::combine_opt(left, right, |_, _| {});
}

#[expect(missing_debug_implementations)]
mod consumer {
    use std::{
        marker::PhantomData,
        ops::ControlFlow,
        sync::atomic::{AtomicBool, Ordering},
    };

    use crate::collector::plumbing;

    pub struct Consumer<'a, T> {
        found: &'a AtomicBool,
        _marker: PhantomData<T>,
    }

    pub struct Combiner(());

    pub struct Serial<'a, T> {
        found: &'a AtomicBool,
        value: Option<T>,
    }

    impl<'a, T> Consumer<'a, T> {
        #[inline]
        pub(super) fn new(found: &'a AtomicBool) -> Self {
            Self {
                found,
                _marker: PhantomData,
            }
        }
    }

    impl<'a, T> plumbing::IntoCollectorBase for Consumer<'a, T> {
        type Output = Option<T>;

        type IntoCollector = Serial<'a, T>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial {
                found: self.found,
                value: None,
            }
        }
    }

    impl<T> plumbing::Consumer for Consumer<'_, T>
    where
        T: Send,
    {
        type Combiner = Combiner;

        plumbing::impl_split_off_left_at_via_unindexed! {}

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            super::max_afford(self.found, request)
        }
    }

    impl<T> plumbing::UnindexedConsumer for Consumer<'_, T>
    where
        T: Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self::new(self.found)
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            Combiner(())
        }
    }

    impl<T> plumbing::Combiner<Option<T>> for Combiner {
        #[inline]
        fn combine(self, left: &mut Option<T>, right: Option<T>) {
            super::combine(left, right);
        }
    }

    impl<T> plumbing::CollectorBase for Serial<'_, T> {
        type Output = Option<T>;

        fn finish(self) -> Self::Output {
            self.value
        }

        plumbing::finish_boxed_impl! {}

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            super::max_afford(self.found, request)
        }
    }

    impl<T> plumbing::Collector<T> for Serial<'_, T> {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            self.found.store(true, Ordering::Relaxed);
            self.value = Some(item);
            ControlFlow::Break(())
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            if self.found.load(Ordering::Relaxed) {
                return ControlFlow::Break(());
            }

            let Some(item) = items.into_iter().next() else {
                return ControlFlow::Continue(());
            };

            self.collect(item)
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::ParAny;

    use crate::test_utils::prelude::*;

    par_collector_test!(indexed {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5_usize);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: ParAny::new(),
        starting_ma_f: |request| (request > 0) as _,
        expected_f: |_, count, request| (
            &nums[..],
            if count > 0 {
                Break(())
            } else {
                Continue((None, (request > 0) as _))
            }
        ),
        output_pred: |res, possible_res| match res {
            None => possible_res.is_empty(),
            Some(res) => possible_res.contains(res),
        },
        state_pred: |collector, expected_state| collector.value == *expected_state,
    });

    unindexed_par_collector_test!(unindexed {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5_usize);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: ParAny::new(),
        starting_ma_f: |request| (request > 0) as _,
        expected_f: |_, count, request| (
            &nums[..],
            if count > 0 {
                Break(())
            } else {
                Continue((None, (request > 0) as _))
            }
        ),
        output_pred: |res, possible_res| match res {
            None => possible_res.is_empty(),
            Some(res) => possible_res.contains(res),
        },
        state_pred: |collector, expected_state| collector.value == *expected_state,
    });
}
