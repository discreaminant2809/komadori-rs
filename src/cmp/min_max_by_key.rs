use std::{fmt::Debug, ops::ControlFlow};

use itertools::MinMaxResult;

use crate::collector::{Collector, CollectorBase};

use super::{MinMax, ValueKey};

/// A collector that computes the minimum and maximum values among the items it collects
/// according to a key-extraction function.
///
/// Its [`Output`](CollectorBase::Output) is:
///
/// - [`MinMaxResult::NoElements`] if no items were collected.
/// - [`MinMaxResult::OneElement`] containing one item if exactly that item was collected.
/// - [`MinMaxResult::MinMax`] containing the minimum and the maximum items (in order)
///   if two or more items were collected.
///
///   If there are multiple equally minimum items, the first one collected is returned.
///   If there are multiple equally maximum items, the last one collected is returned.
///
/// This collector is constructed by [`MinMax::by_key()`](MinMax::by_key).
///
/// This collector corresponds to [`Itertools::minmax_by_key()`](itertools::Itertools::minmax_by_key).
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, cmp::MinMax};
/// use itertools::MinMaxResult;
///
/// assert_eq!(
///     [].into_iter().feed_into(MinMax::by_key(|s: &&str| s.len())),
///     MinMaxResult::NoElements,
/// );
/// assert_eq!(
///     [""].into_iter().feed_into(MinMax::by_key(|s: &&str| s.len())),
///     MinMaxResult::OneElement(""),
/// );
/// assert_eq!(
///     ["noble", "and", "singer"]
///         .into_iter()
///         .feed_into(MinMax::by_key(|s: &&str| s.len())),
///     MinMaxResult::MinMax("and", "singer"),
/// );
/// ```
#[derive(Clone)]
pub struct MinMaxByKey<T, K, F> {
    base: MinMax<ValueKey<T, K>>,
    f: F,
}

impl<T> MinMax<T> {
    /// Creates a new instance of [`MinMaxByKey`] with a given key-extraction function.
    #[inline]
    pub const fn by_key<K, F>(f: F) -> MinMaxByKey<T, K, F>
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        MinMaxByKey {
            base: MinMax::new(),
            f,
        }
    }
}

impl<T, K, F> CollectorBase for MinMaxByKey<T, K, F>
where
    K: Ord,
{
    type Output = MinMaxResult<T>;

    fn finish(self) -> Self::Output {
        let res = self.base.finish();
        unwrap_min_max_res(res)
    }
}

impl<T, K, F> Collector<T> for MinMaxByKey<T, K, F>
where
    F: FnMut(&T) -> K,
    K: Ord,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.base.collect(ValueKey::new(item, &mut self.f))
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.base.collect_many(
            items
                .into_iter()
                .map(|item| ValueKey::new(item, &mut self.f)),
        )
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let Self { base, mut f } = self;

        let res = base.collect_then_finish(
            items
                .into_iter()
                .map(move |item| ValueKey::new(item, &mut f)),
        );

        unwrap_min_max_res(res)
    }
}

fn unwrap_min_max_res<T, K>(res: MinMaxResult<ValueKey<T, K>>) -> MinMaxResult<T> {
    match res {
        MinMaxResult::NoElements => MinMaxResult::NoElements,
        MinMaxResult::OneElement(item) => MinMaxResult::OneElement(item.into_value()),
        MinMaxResult::MinMax(min, max) => MinMaxResult::MinMax(min.into_value(), max.into_value()),
    }
}

impl<T, K, F> Debug for MinMaxByKey<T, K, F>
where
    T: Debug,
    K: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MinMaxByKey")
            .field("state", self.base.debug_state())
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use itertools::Itertools;

    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    use super::super::test_utils::Id;
    use super::*;

    proptest! {
        #[test]
        fn all_collect_methods(
            nums in propvec(any::<i32>(), ..=3),
            starting_nums in propvec(any::<i32>(), ..=3),
        ) {
            all_collect_methods_impl(nums, starting_nums)?;
        }
    }

    fn all_collect_methods_impl(nums: Vec<i32>, starting_nums: Vec<i32>) -> TestCaseResult {
        fn key_extractor(Id { num, .. }: &Id) -> i32 {
            num.wrapping_add(i32::MAX)
        }

        BasicCollectorTester {
            iter_factory: || nums.iter().enumerate().map(|(id, &num)| Id { id, num }),
            collector_factory: || {
                let mut collector = MinMax::by_key(key_extractor);
                let _ = collector.collect_many(
                    starting_nums
                        .iter()
                        .zip(nums.len()..)
                        .map(|(&num, id)| Id { id, num }),
                );
                collector
            },
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                let iter = starting_nums
                    .iter()
                    .zip(nums.len()..)
                    .map(|(&num, id)| Id { id, num })
                    .chain(iter);

                if !Id::full_eq_minmax_res(iter.minmax_by_key(key_extractor), output) {
                    Err(PredError::IncorrectOutput)
                } else if remaining.next().is_some() {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }
}
