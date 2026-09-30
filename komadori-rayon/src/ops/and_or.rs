use std::{
    ops::ControlFlow,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, OpaqueUnindexedConsumer, SerialOf,
            SerialOutputOf, UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    helpers::{unique, unique_unindexed},
};

/// A collector that `&&`s all collected `bool`s, and stops when it encounters a `false` *anywhere*.
///
/// Its [`Output`] is `true` as long as it only collects `true`, and `false` if
/// it collects `false` once *anywhere* among the collected items.
/// If no items were collected, the [`Output`] is `true`.
///
/// `ParAnd::new().map(f)` corresponds to [`ParallelIterator::all(f)`].
///
/// [`Output`]: ParallelCollectorBase::Output
/// [`ParallelIterator::all(f)`]: rayon::iter::ParallelIterator::all
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, ops::ParAnd};
///
/// let all_true = [true, true, true]
///     .into_par_iter()
///     .feed_into(ParAnd::new());
///
/// assert!(all_true);
/// ```
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, ops::ParAnd};
///
/// let all_true = [true, false, true]
///     .into_par_iter()
///     .feed_into(ParAnd::new());
///
/// assert!(!all_true);
/// ```
///
/// Most of the time, this parallel collector is paired with [`map()`](ParallelCollectorBase::map):
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, ops::ParAnd};
///
/// let all_positive = [1, 9, 2]
///     .into_par_iter()
///     .feed_into(ParAnd::new().map(|num| num > 0));
///
/// assert!(all_positive);
/// ```
#[derive(Debug)]
pub struct ParAnd(AtomicBool);

/// A collector that `||`s all collected `bool`s, and stops when it encounters a `true` *anywhere*.
///
/// Its [`Output`] is `false` as long as it only collects `false`, and `true` if
/// it collects `true` once *anywhere* among the collected items.
/// If no items were collected, the [`Output`] is `false`.
///
/// `ParOr::new().map(f)` corresponds to [`ParallelIterator::any(f)`].
///
/// [`Output`]: ParallelCollectorBase::Output
/// [`ParallelIterator::any(f)`]: rayon::iter::ParallelIterator::any
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, ops::ParOr};
///
/// let any_true = [false, false, false]
///     .into_par_iter()
///     .feed_into(ParOr::new());
///
/// assert!(!any_true);
/// ```
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, ops::ParOr};
///
/// let any_true = [false, true, false]
///     .into_par_iter()
///     .feed_into(ParOr::new());
///
/// assert!(any_true);
/// ```
///
/// Most of the time, this parallel collector is paired with [`map()`](ParallelCollectorBase::map):
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, ops::ParOr};
///
/// let any_positive = [0, -1, 7]
///     .into_par_iter()
///     .feed_into(ParOr::new().map(|num| num > 0));
///
/// assert!(any_positive);
/// ```
#[derive(Debug)]
pub struct ParOr(AtomicBool);

impl ParAnd {
    /// Creates a new instance of this parallel collector
    /// with an initial value of `true`.
    #[inline]
    pub const fn new() -> Self {
        assert_unindexed_par_collector::<_, bool>(Self(AtomicBool::new(true)))
    }
}

impl ParOr {
    /// Creates a new instance of this parallel collector
    /// with an initial value of `false`.
    #[inline]
    pub const fn new() -> Self {
        assert_unindexed_par_collector::<_, bool>(Self(AtomicBool::new(false)))
    }
}

impl Default for ParAnd {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ParOr {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ParAnd {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.load(Ordering::Relaxed).into())
    }
}

impl Clone for ParOr {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.load(Ordering::Relaxed).into())
    }
}

impl<'a> DefineSerial<'a> for ParAnd {
    type Serial = unique::Serial<'a, Self, consumer::Serial<'a, true>>;
}

impl<'a> DefineSerial<'a> for ParOr {
    type Serial = unique::Serial<'a, Self, consumer::Serial<'a, false>>;
}

impl<'a> DefineUnindexedSerial<'a> for ParAnd {
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial<'a, true>>;
}

impl<'a> DefineUnindexedSerial<'a> for ParOr {
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial<'a, false>>;
}

impl ParallelCollectorBase for ParAnd {
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0.into_inner()
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        max_afford::<true>(&self.0, request)
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify(parts(&self.0))
    }

    fn take_parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        unique::take_uniquify(take_parts(&self.0))
    }
}

impl ParallelCollectorBase for ParOr {
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0.into_inner()
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        max_afford::<false>(&self.0, request)
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify(parts(&self.0))
    }

    fn take_parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        unique::take_uniquify(take_parts(&self.0))
    }
}

impl UnindexedParallelCollectorBase for ParAnd {
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify(parts(&self.0))
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
        unique_unindexed::take_uniquify(take_parts(&self.0))
    }
}

impl UnindexedParallelCollectorBase for ParOr {
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify(parts(&self.0))
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
        unique_unindexed::take_uniquify(take_parts(&self.0))
    }
}

#[inline]
fn max_afford<const IS_AND: bool>(flag: &AtomicBool, request: usize) -> usize {
    if flag.load(Ordering::Relaxed) ^ IS_AND {
        0
    } else {
        request
    }
}

#[inline]
fn parts<const IS_AND: bool>(
    flag: &AtomicBool,
) -> (
    OpaqueUnindexedConsumer!(consumer::Serial<'_, IS_AND>),
    impl FnOnce(()) -> ControlFlow<()>,
) {
    (consumer::unindexed(flag), |_| {
        if flag.load(Ordering::Relaxed) ^ IS_AND {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
}

#[inline]
fn take_parts<const IS_AND: bool>(
    flag: &AtomicBool,
) -> (
    OpaqueUnindexedConsumer!(consumer::Serial<'_, IS_AND>),
    impl FnOnce(()),
) {
    (consumer::unindexed(flag), |_| {})
}

#[expect(missing_debug_implementations)]
mod consumer {
    use std::{
        ops::ControlFlow,
        sync::atomic::{AtomicBool, Ordering},
    };

    use crate::collector::plumbing::{self, BasicUnindexedConsumer, OpaqueUnindexedConsumer};

    pub fn unindexed<const IS_AND: bool>(
        flag: &AtomicBool,
    ) -> OpaqueUnindexedConsumer!(Serial<'_, IS_AND>) {
        BasicUnindexedConsumer {
            state: flag,
            split_f: Clone::clone,
            combiner_f: |_| |_, _| {},
            ma_f: |flag, request| super::max_afford::<IS_AND>(flag, request),
            collector_f: Serial,
        }
    }

    pub struct Serial<'a, const IS_AND: bool>(&'a AtomicBool);

    impl<const IS_AND: bool> plumbing::CollectorBase for Serial<'_, IS_AND> {
        type Output = ();

        #[inline]
        fn finish(self) -> Self::Output {}

        plumbing::finish_boxed_impl! {}

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            super::max_afford::<IS_AND>(self.0, request)
        }
    }

    impl<const IS_AND: bool> plumbing::Collector<bool> for Serial<'_, IS_AND> {
        #[inline]
        fn collect(&mut self, item: bool) -> ControlFlow<()> {
            // See the `par_or` benchmark for why we use two separate
            // `load` and `store` instead of `fetch_*`.
            if IS_AND {
                if !self.0.load(Ordering::Relaxed) {
                    ControlFlow::Break(())
                } else if !item {
                    self.0.store(false, Ordering::Relaxed);
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            } else {
                if self.0.load(Ordering::Relaxed) {
                    ControlFlow::Break(())
                } else if item {
                    self.0.store(true, Ordering::Relaxed);
                    ControlFlow::Break(())
                } else {
                    ControlFlow::Continue(())
                }
            }
        }
    }
}

#[cfg(test)]
mod proptests {
    use std::{convert::identity, sync::atomic::Ordering};

    use super::{ParAnd, ParOr};

    use crate::test_utils::prelude::*;

    par_collector_test!(indexed_and {
        iter_data: {
            let mut nums = propvec(any::<bool>(), ..=5_usize);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: ParAnd::new(),
        starting_ma_f: |request| request,
        expected_f: |mut iter, _, request| {
            let all_true = iter.all(identity);

            (
                all_true,
                if all_true {
                    Continue((all_true, request))
                } else {
                    Break(())
                },
            )
        },
        output_pred: PartialEq::eq,
        state_pred: |collector, &expected_flag| collector.0.load(Ordering::Relaxed)
            == expected_flag,
    });

    unindexed_par_collector_test!(unindexed_and {
        iter_data: {
            let mut nums = propvec(any::<bool>(), ..=5_usize);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: ParAnd::new(),
        starting_ma_f: |request| request,
        expected_f: |mut iter, _, request| {
            let all_true = iter.all(identity);

            (
                all_true,
                if all_true {
                    Continue((all_true, request))
                } else {
                    Break(())
                },
            )
        },
        output_pred: PartialEq::eq,
        state_pred: |collector, &expected_flag| collector.0.load(Ordering::Relaxed)
            == expected_flag,
    });

    par_collector_test!(indexed_or {
        iter_data: {
            let mut nums = propvec(any::<bool>(), ..=5_usize);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: ParOr::new(),
        starting_ma_f: |request| request,
        expected_f: |mut iter, _, request| {
            let any_true = iter.any(identity);

            (
                any_true,
                if any_true {
                    Break(())
                } else {
                    Continue((any_true, request))
                },
            )
        },
        output_pred: PartialEq::eq,
        state_pred: |collector, &expected_flag| collector.0.load(Ordering::Relaxed)
            == expected_flag,
    });

    unindexed_par_collector_test!(unindexed_or {
        iter_data: {
            let mut nums = propvec(any::<bool>(), ..=5_usize);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: ParOr::new(),
        starting_ma_f: |request| request,
        expected_f: |mut iter, _, request| {
            let any_true = iter.any(identity);

            (
                any_true,
                if any_true {
                    Break(())
                } else {
                    Continue((any_true, request))
                },
            )
        },
        output_pred: PartialEq::eq,
        state_pred: |collector, &expected_flag| collector.0.load(Ordering::Relaxed)
            == expected_flag,
    });
}
