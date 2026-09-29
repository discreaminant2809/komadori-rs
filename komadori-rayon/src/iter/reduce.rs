use std::{fmt::Debug, ops::ControlFlow};

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

/// A parallel collector that reduces all collected items into a single value
/// by repeatedly applying a reduction function.
///
/// If no items have been collected, its [`Output`](ParallelCollectorBase::Output) is `None`;
/// otherwise, it returns `Some` containing the result of the reduction.
///
/// This collector corresponds to [`Iterator::reduce()`], except the closure is
/// the "left" value mutated by the "right" value instead of the two values
/// producing another value. Also, the application order is unspecified rather
/// than strictly from left to right, but it is still guaranteed that when
/// two items are fed into the closure, the first one is left compared to
/// the second one (the "right" value).
///
/// This parallel collector collects `T`.
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParReduce};
///
/// let res = [3, 2, 5, 1, 4]
///     .into_par_iter()
///     .feed_into(ParReduce::new(|accum, num| *accum += num));
///
/// assert_eq!(res, Some(15));
/// ```
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParReduce};
///
/// let res = ([] as [i32; _])
///     .into_par_iter()
///     .feed_into(ParReduce::new(|accum, num| *accum += num));
///
/// assert_eq!(res, None);
/// ```
#[derive(Clone)]
pub struct ParReduce<T, F> {
    accum: Option<T>,
    f: F,
}

impl<T: Debug, F> Debug for ParReduce<T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reduce")
            .field("accum", &self.accum)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

impl<T, F> ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    /// Creates a new instance of this parallel collector with a given accumulator.
    #[inline]
    pub const fn new(f: F) -> Self {
        assert_unindexed_par_collector::<_, T>(Self { accum: None, f })
    }
}

impl<'this, T, F> DefineSerial<'this> for ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    type Serial = unique::Serial<'this, Self, consumer::Serial<T, &'this F>>;
}

impl<T, F> ParallelCollectorBase for ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.accum
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((consumer::unindexed(&self.f), |output| {
            crate::iter::combine_opt(&mut self.accum, output, &self.f);
            ControlFlow::Continue(())
        }))
    }
}

impl<'this, T, F> DefineUnindexedSerial<'this> for ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, consumer::Serial<T, &'this F>>;
}

impl<T, F> UnindexedParallelCollectorBase for ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
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
        unique_unindexed::uniquify((consumer::unindexed(&self.f), |output| {
            crate::iter::combine_opt(&mut self.accum, output, &self.f);
            ControlFlow::Continue(())
        }))
    }
}

mod consumer {
    use crate::collector::plumbing::{BasicUnindexedConsumer, OpaqueUnindexedConsumer};

    pub fn unindexed<T, F>(f: F) -> OpaqueUnindexedConsumer!(Serial<T, F>)
    where
        T: Send,
        F: FnMut(&mut T, T) + Clone + Send,
    {
        BasicUnindexedConsumer {
            state: f,
            split_f: Clone::clone,
            combiner_f: |f| {
                let f = f.clone();
                |left, right| crate::iter::combine_opt(left, right, f)
            },
            ma_f: |_, request| request,
            collector_f: Serial::new,
        }
    }

    pub type Serial<T, F> = komadori::iter::Reduce<T, F>;
}

#[cfg(test)]
mod proptests {
    use std::ops::RangeInclusive;

    use super::ParReduce;

    use crate::test_utils::prelude::*;

    // Won't overflow since we only add up to ±300,000,000 * 6 = ±1,800,000,000.
    const NUM_RANGE: RangeInclusive<i32> = -300_000_000..=300_000_000;

    par_collector_test!(indexed {
        iter_data: {
            let mut nums = propvec(NUM_RANGE, ..=5);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: ParReduce::new(|a, b| *a += b),
        starting_ma_f: |request| request,
        expected_f: |iter, _, request| (iter.reduce(|a, b| a + b), Continue(((), request))),
        output_pred: PartialEq::eq,
        state_pred: state_is_irrelevant(),
    });

    unindexed_par_collector_test!(unindexed {
        iter_data: {
            let mut nums = propvec(NUM_RANGE, ..=5);
        },
        other_data: {},
        iter: nums.par_iter().cloned(),
        collector: ParReduce::new(|a, b| *a += b),
        starting_ma_f: |request| request,
        expected_f: |iter, _, request| (iter.reduce(|a, b| a + b), Continue(((), request))),
        output_pred: PartialEq::eq,
        state_pred: state_is_irrelevant(),
    });
}
