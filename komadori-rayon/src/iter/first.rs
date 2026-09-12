#![forbid(unsafe_code, reason = "Prevent misuses of `MaybeUninit`")]

use std::{mem::MaybeUninit, ops::ControlFlow, sync::atomic::AtomicUsize};

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

/// A parallel collector that finds the "first" (left-most) value
/// out of all items it collected.
///
/// If it collected one item, its [`Output`] is [`Some(item)`](Some)
/// containing the "left-most" item, otherwise [`None`].
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
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParFirst};
///
/// let first = [1, 2, 3]
///     .into_par_iter()
///     .feed_into(ParFirst::new());
///
/// assert_eq!(first, Some(1));
/// ```
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParFirst};
///
/// let first = ([] as [i32; _])
///     .into_par_iter()
///     .feed_into(ParFirst::new());
///
/// assert_eq!(first, None);
/// ```
///
/// `ParFirst::new().filter(f)` corresponds to
/// [`ParallelIterator::find_first(f)`]:
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParFirst};
///
/// let first = [1, 5, 6, 3, 8]
///     .into_par_iter()
///     .feed_into(ParFirst::new().filter(|&num| num % 2 == 0));
///
/// assert_eq!(first, Some(6));
/// ```
///
/// `ParFirst::new().filter_map(f)` corresponds to
/// [`ParallelIterator::find_map_first(f)`]:
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParFirst};
///
/// let first = ["noble", "and", "1", "singer", "2"]
///     .into_par_iter()
///     .feed_into(ParFirst::new().filter_map(|s: &str| s.parse().ok()));
///
/// assert_eq!(first, Some(1));
/// ```
///
/// [`Output`]: ParallelCollectorBase
/// [`ParallelIterator::find_first(f)`]: rayon::iter::ParallelIterator::find_first
/// [`ParallelIterator::find_map_first(f)`]: rayon::iter::ParallelIterator::find_map_first
#[derive(Debug)]
pub struct ParFirst<T> {
    value: Option<T>,
    // This field is only "activated" in the unindexed path
    // and won't be touched in the indexed path.
    //
    // Worry not, we won't be using any `unsafe` code here!
    // `AtomicUsize` doesn't need dropping anyway.
    best_found: MaybeUninit<AtomicUsize>,
}

impl<T> ParFirst<T>
where
    T: Send,
{
    /// Creates a new instance of this parallel collector.
    #[inline]
    pub const fn new() -> Self {
        assert_unindexed_par_collector::<_, T>(Self {
            value: None,
            best_found: MaybeUninit::uninit(),
        })
    }
}

impl<T> Clone for ParFirst<T>
where
    T: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            best_found: MaybeUninit::uninit(),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.value.clone_from(&source.value);
    }
}

impl<T> Default for ParFirst<T>
where
    T: Send,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> DefineSerial<'a> for ParFirst<T>
where
    T: Send,
{
    type Serial = unique::Serial<'a, Self, indexed::Serial<T>>;
}

impl<'a, T> DefineUnindexedSerial<'a> for ParFirst<T>
where
    T: Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, unindexed::Serial<'a, T>>;
}

impl<T> ParallelCollectorBase for ParFirst<T>
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
        request.min(self.value.is_none() as _)
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((indexed::Consumer::new(), |output| {
            combine(&mut self.value, output);
            if self.value.is_some() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }))
    }
}

impl<T> UnindexedParallelCollectorBase for ParFirst<T>
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
        let best_found = self.best_found.write(AtomicUsize::new(usize::MAX));

        unique_unindexed::uniquify((unindexed::Consumer::new(best_found), |output| {
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
fn combine<T>(left: &mut Option<T>, right: Option<T>) {
    // We don't care what the right is, since we follow the "first" semantic.
    super::combine_opt(left, right, |_, _| {});
}

#[expect(missing_debug_implementations)]
mod indexed {
    use std::{marker::PhantomData, ops::ControlFlow};

    use crate::collector::plumbing;

    pub struct Consumer<T> {
        disabled: bool,
        _marker: PhantomData<T>,
    }

    pub struct Combiner(());

    pub enum Serial<T> {
        NotYet,
        First(T),
        Disabled,
    }

    impl<T> Consumer<T> {
        #[inline]
        pub(super) fn new() -> Self {
            Self {
                disabled: false,
                _marker: PhantomData,
            }
        }
    }

    impl<T> plumbing::IntoCollectorBase for Consumer<T> {
        type Output = Option<T>;

        type IntoCollector = Serial<T>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            if self.disabled {
                Serial::Disabled
            } else {
                Serial::NotYet
            }
        }
    }

    impl<T> plumbing::Consumer for Consumer<T>
    where
        T: Send,
    {
        type Combiner = Combiner;

        #[inline]
        fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner) {
            let disabled = self.disabled;
            // It means that the better "first" value can be found at the left.
            if index > 0 {
                self.disabled = true;
            }

            (
                Self {
                    disabled,
                    _marker: PhantomData,
                },
                Combiner(()),
            )
        }

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            request.min(!self.disabled as _)
        }
    }

    // Note: In practice we will only have at most one `Some` globally.
    impl<T> plumbing::Combiner<Option<T>> for Combiner {
        #[inline]
        fn combine(self, left: &mut Option<T>, right: Option<T>) {
            super::combine(left, right);
        }
    }

    impl<T> plumbing::CollectorBase for Serial<T> {
        type Output = Option<T>;

        fn finish(self) -> Self::Output {
            match self {
                Self::NotYet | Self::Disabled => None,
                Self::First(value) => Some(value),
            }
        }

        plumbing::finish_boxed_impl! {}

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            request.min(matches!(self, Self::NotYet) as _)
        }
    }

    impl<T> plumbing::Collector<T> for Serial<T> {
        // Note: Unless disabled,
        // we assume that the callers uphold the contracts of collectors,
        // which (mathematically) means that those methods are called
        // when the state is `NotYet`.

        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            if !matches!(self, Self::Disabled) {
                *self = Self::First(item);
            }

            ControlFlow::Break(())
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            if !matches!(self, Self::Disabled)
                && let Some(item) = items.into_iter().next()
            {
                *self = Self::First(item);
            }

            if matches!(self, Self::First(_)) {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }
    }
}

#[expect(missing_debug_implementations)]
mod unindexed {
    use std::{
        cell::Cell,
        marker::PhantomData,
        ops::ControlFlow,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use crate::collector::plumbing::{self, UnindexedConsumer};

    pub struct Consumer<'a, T> {
        lower: Cell<usize>,
        upper: usize,
        best_found: &'a AtomicUsize,
        _marker: PhantomData<T>,
    }

    pub struct Combiner(());

    pub struct Serial<'a, T> {
        pos: usize,
        best_found: &'a AtomicUsize,
        value: Option<T>,
    }

    impl<'a, T> Consumer<'a, T> {
        #[inline]
        pub(super) fn new(best_found: &'a AtomicUsize) -> Self {
            Self {
                lower: 0.into(),
                upper: usize::MAX,
                best_found,
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
                pos: self.lower.get(),
                best_found: self.best_found,
                value: None,
            }
        }
    }

    impl<'a, T> plumbing::Consumer for Consumer<'a, T>
    where
        T: Send,
    {
        type Combiner = Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _index: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            // We can still afford if the best found is still at the right leaves.
            (request > 0 && self.best_found.load(Ordering::Relaxed) >= self.lower.get()) as _
        }
    }

    impl<'a, T> plumbing::UnindexedConsumer for Consumer<'a, T>
    where
        T: Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            let lower = self.lower.get();
            let upper = self.upper.midpoint(lower);
            self.lower.set(upper);

            Self {
                lower: lower.into(),
                upper,
                best_found: self.best_found,
                _marker: PhantomData,
            }
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

    impl<'a, T> plumbing::CollectorBase for Serial<'a, T> {
        type Output = Option<T>;

        #[inline]
        fn finish(self) -> Self::Output {
            self.value
        }

        plumbing::finish_boxed_impl! {}

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            // We can still afford if the best found is still at the right leaves.
            (request > 0 && self.best_found.load(Ordering::Relaxed) >= self.pos) as _
        }
    }

    impl<'a, T> plumbing::Collector<T> for Serial<'a, T> {
        // Note: We assume that the callers uphold the contracts of collectors,
        // which (mathematically) means that those methods are called
        // when `self.value` is `None`.

        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            // Note: `fetch_min` returns the previous value, not the min/max value.
            // So we have two cases:
            // - best_found < self.pos: The return value is smaller
            //   => Shouldn't update self.value
            //   => Also means the left-most leaves have found the better "first" value.
            // - best_found >= self.pos: The return value is greater or equal to
            //   => Should update self.value
            if self.best_found.fetch_min(self.pos, Ordering::Relaxed) >= self.pos {
                self.value = Some(item);
            }

            ControlFlow::Break(())
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            if self.best_found.load(Ordering::Relaxed) < self.pos {
                return ControlFlow::Break(());
            }

            let Some(item) = items.into_iter().next() else {
                return ControlFlow::Continue(());
            };

            if self.best_found.fetch_min(self.pos, Ordering::Relaxed) >= self.pos {
                self.value = Some(item);
            }

            ControlFlow::Break(())
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::ParFirst;

    use crate::test_utils::prelude::*;

    par_collector_test!(indexed {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5_usize);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: ParFirst::new(),
        starting_ma_f: |request| (request > 0) as _,
        expected_f: |mut iter, count, request| (
            iter.next(),
            if count > 0 {
                Break(())
            } else {
                Continue((None, (request > 0) as _))
            }
        ),
        output_pred: PartialEq::eq,
        state_pred: |collector, expected_state| collector.value == *expected_state,
    });

    unindexed_par_collector_test!(unindexed {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5_usize);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: ParFirst::new(),
        starting_ma_f: |request| (request > 0) as _,
        expected_f: |mut iter, count, request| (
            iter.next(),
            if count > 0 {
                Break(())
            } else {
                Continue((None, (request > 0) as _))
            }
        ),
        output_pred: PartialEq::eq,
        state_pred: |collector, expected_state| collector.value == *expected_state,
    });
}
