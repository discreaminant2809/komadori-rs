use core::ops::ControlFlow;

use either::{Either, for_both};

use crate::collector::{Collector, CollectorBase, finish_boxed_impl};

/// [`Either`] is a collector when both branches are so.
/// The left and the right collectors must finish with
/// the same [`Output`](CollectorBase::Output),
/// and [`Either`] collects `T` when both branches can collect `T`.
///
/// This is useful when you want to return different collector types dynamically
/// based on a condition without using trait objects.
///
/// # Examples
///
/// ```
/// use komadori::{
///     prelude::*,
///     either::Either,
///     cmp::{Max, Min},
/// };
///
/// enum Extreme {
///     Max,
///     Min,
/// }
///
/// fn stats(
///     extreme: Extreme,
/// ) -> impl Collector<i32, Output = (i32, Option<i32>)> {
///     (
///         0.into_sum(),
///         match extreme {
///             Extreme::Max => Either::Left(Max::new()),
///             Extreme::Min => Either::Right(Min::new()),
///         }
///     )
///     .into_collector()
/// }
///
/// let nums = [1, 3, 2];
///
/// assert_eq!(
///     nums.into_iter().feed_into(stats(Extreme::Max)),
///     (6, Some(3)),
/// );
///
/// assert_eq!(
///     nums.into_iter().feed_into(stats(Extreme::Min)),
///     (6, Some(1)),
/// );
/// ```
impl<L, R> CollectorBase for Either<L, R>
where
    L: CollectorBase,
    R: CollectorBase<Output = L::Output>,
{
    type Output = L::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        for_both!(self, collector => collector.finish())
    }

    finish_boxed_impl! {}

    #[inline]
    fn reserve(&mut self, additional: usize) {
        for_both!(self, collector => collector.reserve(additional));
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        for_both!(self, collector => collector.max_afford(request))
    }
}

impl<L, R, T> Collector<T> for Either<L, R>
where
    L: Collector<T>,
    R: Collector<T, Output = L::Output>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        for_both!(self, collector => collector.collect(item))
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        for_both!(self, collector => collector.collect_many(items))
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        for_both!(self, collector => collector.collect_then_finish(items))
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: The caller has reserved for one item.
            for_both!(self, collector => collector.assume_reserved_collect(item))
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::{collector::take_collector_model, either::Either, test_utils::prelude::*};

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let is_left = any::<bool>();
            let n = ..=5_usize;
        },
        iter: nums.iter().copied(),
        collector: if is_left {
            Either::Left(vec![].into_collector().take(n))
        } else {
            Either::Right(vec![].into_collector().take(n))
        },
        expected_f: |iter, count| {
            let res: Vec<_> = iter.take(n).collect();
            (res, count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });
}
