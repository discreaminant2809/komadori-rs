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

/// A parallel collector that finds the "last" (right-most) value
/// out of all items it collected.
///
/// If it collected one item, its [`Output`] is [`Some(item)`](Some)
/// containing the "right-most" item, otherwise [`None`].
///
/// This parallel collector collects `T`.
///
/// This parallel collector might seem silly, but it enables many patterns,
/// especially when combining with some adapters.
/// See examples for more.
///
/// # Notes
///
/// Unlike [`komadori::iter::Last`], this parallel collector may not process
/// all items.
// TODO: When done implementing `ParallelCollectorBase::chain`, add:
// "If you want all the side effects to be run, use [`ParallelCollectorBase::chain`]."
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParLast};
///
/// let first = [1, 2, 3]
///     .into_par_iter()
///     .feed_into(ParLast::new());
///
/// assert_eq!(first, Some(3));
/// ```
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParLast};
///
/// let first = ([] as [i32; _])
///     .into_par_iter()
///     .feed_into(ParLast::new());
///
/// assert_eq!(first, None);
/// ```
///
/// `ParLast::new().filter(f)` corresponds to
/// [`ParallelIterator::find_last(f)`]:
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParLast};
///
/// let first = [1, 6, 5, 8, 3]
///     .into_par_iter()
///     .feed_into(ParLast::new().filter(|&num| num % 2 == 0));
///
/// assert_eq!(first, Some(8));
/// ```
///
/// `ParLast::new().filter_map(f)` corresponds to
/// [`ParallelIterator::find_map_last(f)`]:
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParLast};
///
/// let first = ["noble", "1", "and", "2", "singer"]
///     .into_par_iter()
///     .feed_into(ParLast::new().filter_map(|s: &str| s.parse().ok()));
///
/// assert_eq!(first, Some(2));
/// ```
///
/// [`Output`]: ParallelCollectorBase
/// [`ParallelIterator::find_last(f)`]: rayon::iter::ParallelIterator::find_last
/// [`ParallelIterator::find_map_last(f)`]: rayon::iter::ParallelIterator::find_map_last
#[derive(Debug)]
pub struct ParLast<T> {
    value: Option<T>,
    // This field is only "activated" in the unindexed path
    // and won't be touched in the indexed path.
    //
    // Worry not, we won't be using any `unsafe` code here!
    // `AtomicUsize` doesn't need dropping anyway.
    best_found: MaybeUninit<AtomicUsize>,
}

impl<T> ParLast<T>
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

impl<T> Clone for ParLast<T>
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

impl<T> Default for ParLast<T>
where
    T: Send,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> DefineSerial<'a> for ParLast<T>
where
    T: Send,
{
    type Serial = unique::Serial<'a, Self, indexed::Serial<T>>;
}

impl<'a, T> DefineUnindexedSerial<'a> for ParLast<T>
where
    T: Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, unindexed::Serial<'a, T>>;
}

impl<T> ParallelCollectorBase for ParLast<T>
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
        request
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((indexed::Consumer::new(len), |output| {
            combine(&mut self.value, output);
            ControlFlow::Continue(())
        }))
    }
}

impl<T> UnindexedParallelCollectorBase for ParLast<T>
where
    T: Send,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        let best_found = self.best_found.write(AtomicUsize::new(0));

        unique_unindexed::uniquify((unindexed::Consumer::new(best_found), |output| {
            combine(&mut self.value, output);
            ControlFlow::Continue(())
        }))
    }
}

#[inline]
fn combine<T>(left: &mut Option<T>, right: Option<T>) {
    super::combine_opt(left, right, |left, right| *left = right);
}

#[expect(missing_debug_implementations)]
mod indexed {
    use std::{marker::PhantomData, num::NonZero, ops::ControlFlow};

    use crate::collector::plumbing;

    pub struct Consumer<T> {
        // We track the len so that we can compare it to `index`
        // and see whether to disable or not.
        // `None` means this leaf is "disabled" and won't collect items.
        // Note that `0` len means being disabled as well, so we use
        // `Option<NonZero<usize>>`.
        len: Option<NonZero<usize>>,
        _marker: PhantomData<T>,
    }

    pub struct Combiner(());

    pub enum Serial<T> {
        NotYet,
        Last(T),
        Disabled,
    }

    impl<T> Consumer<T> {
        #[inline]
        pub(super) fn new(len: usize) -> Self {
            Self {
                len: NonZero::new(len),
                _marker: PhantomData,
            }
        }
    }

    impl<T> plumbing::IntoCollectorBase for Consumer<T> {
        type Output = Option<T>;

        type IntoCollector = Serial<T>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            if self.len.is_none() {
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
            let len = (|| {
                self.len = NonZero::new(self.len?.get() - index);
                if self.len.is_some() {
                    None
                } else {
                    NonZero::new(index)
                }
            })();

            (
                Self {
                    len,
                    _marker: PhantomData,
                },
                Combiner(()),
            )
        }

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            if self.len.is_some() { request } else { 0 }
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
                Self::Last(value) => Some(value),
            }
        }

        plumbing::finish_boxed_impl! {}

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            if matches!(self, Self::Disabled) {
                0
            } else {
                request
            }
        }
    }

    impl<T> plumbing::Collector<T> for Serial<T> {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            if matches!(self, Self::Disabled) {
                ControlFlow::Break(())
            } else {
                *self = Self::Last(item);
                ControlFlow::Continue(())
            }
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            match self {
                Self::Disabled => return ControlFlow::Break(()),
                Self::NotYet => {
                    if let Some(last) = items.into_iter().last() {
                        *self = Self::Last(last)
                    }
                }
                Self::Last(left) => {
                    if let Some(right) = items.into_iter().last() {
                        *left = right
                    }
                }
            }

            ControlFlow::Continue(())
        }

        #[inline]
        fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
            match self {
                Self::Disabled => None,
                Self::NotYet => items.into_iter().last(),
                Self::Last(left) => items.into_iter().last().or(Some(left)),
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

    use crate::collector::plumbing;

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
                pos: self.upper,
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
            use plumbing::UnindexedConsumer;
            (self.split_off_left(), self.to_combiner())
        }

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            // We can still afford if the best found is still at the left leaves.
            if self.best_found.load(Ordering::Relaxed) <= self.upper {
                request
            } else {
                0
            }
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
            // We can still afford if the best found is still at the left leaves.
            if self.best_found.load(Ordering::Relaxed) <= self.pos {
                request
            } else {
                0
            }
        }
    }

    impl<'a, T> plumbing::Collector<T> for Serial<'a, T> {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            // Note: `fetch_max` returns the previous value, not the min/max value.
            // So we have two cases:
            // - best_found > self.pos: The return value is greater
            //   => Shouldn't update self.value
            //   => Also means the right-side leaves have found the better "last" value.
            // - best_found <= self.pos: The return value is smaller or equal to
            //   => Should update self.value
            if self.best_found.fetch_max(self.pos, Ordering::Relaxed) <= self.pos {
                self.value = Some(item);
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::ParLast;

    use crate::test_utils::prelude::*;

    par_collector_test!(indexed {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5_usize);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: ParLast::new(),
        starting_ma_f: |request| request,
        expected_f: |iter, _, request| {
            let res = iter.last();
            (res, Continue((res, request)))
        },
        output_pred: PartialEq::eq,
        state_pred: |collector, expected_state| collector.value == *expected_state,
    });

    unindexed_par_collector_test!(unindexed {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5_usize);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: ParLast::new(),
        starting_ma_f: |request| request,
        expected_f: |iter, _, request| {
            let res = iter.last();
            (res, Continue((res, request)))
        },
        output_pred: PartialEq::eq,
        state_pred: |collector, expected_state| collector.value == *expected_state,
    });
}
