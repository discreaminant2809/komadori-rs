//! [`Collector`]s for [`Vec`].
//!
//! This module corresponds to [`mod@std::vec`].

use crate::{
    collector::{Collector, CollectorBase},
    slice::{Concat, ConcatItem, ConcatItemSealed, ConcatSealed},
};

use std::{borrow::Borrow, ops::ControlFlow};

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;

/// A collector that pushes collected items into a [`Vec`].
/// Its [`Output`] is [`Vec`].
///
/// This struct is created by `Vec::into_collector()`.
///
/// [`Output`]: CollectorBase::Output
#[derive(Debug, Clone)]
pub struct IntoCollector<T>(Vec<T>);

/// A collector that pushes collected items into a [`&mut Vec`](Vec).
/// Its [`Output`] is [`&mut Vec`](Vec).
///
/// This struct is created by `Vec::collector_mut()`.
///
/// [`Output`]: CollectorBase::Output
#[derive(Debug)]
pub struct CollectorMut<'a, T>(&'a mut Vec<T>);

impl<T> crate::collector::IntoCollectorBase for Vec<T> {
    type Output = Self;

    type IntoCollector = IntoCollector<T>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        IntoCollector(self)
    }
}

impl<'a, T> crate::collector::IntoCollectorBase for &'a mut Vec<T> {
    type Output = Self;

    type IntoCollector = CollectorMut<'a, T>;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        CollectorMut(self)
    }
}

impl<T> CollectorBase for IntoCollector<T> {
    type Output = Vec<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }
}

impl<T> Collector<T> for IntoCollector<T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.0.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.0.extend(items);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.0.extend(items);
        self.0
    }
}

impl<'i, T> Collector<&'i T> for IntoCollector<T>
where
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &item: &'i T) -> ControlFlow<()> {
        self.0.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i T>) -> ControlFlow<()> {
        self.0.extend(items);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = &'i T>) -> Self::Output {
        self.0.extend(items);
        self.0
    }
}

impl<'i, T> Collector<&'i mut T> for IntoCollector<T>
where
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &mut item: &'i mut T) -> ControlFlow<()> {
        self.0.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i mut T>) -> ControlFlow<()> {
        self.0.extend(items.into_iter().map(|&mut item| item));
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = &'i mut T>) -> Self::Output {
        self.0.extend(items.into_iter().map(|&mut item| item));
        self.0
    }
}

impl<'a, T> CollectorBase for CollectorMut<'a, T> {
    type Output = &'a mut Vec<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }
}

impl<'a, T> Collector<T> for CollectorMut<'a, T> {
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        self.0.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        self.0.extend(items);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        self.0.extend(items);
        self.0
    }
}

impl<'a, 'i, T> Collector<&'i T> for CollectorMut<'a, T>
where
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &item: &'i T) -> ControlFlow<()> {
        self.0.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i T>) -> ControlFlow<()> {
        self.0.extend(items);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = &'i T>) -> Self::Output {
        self.0.extend(items);
        self.0
    }
}

impl<'a, 'i, T> Collector<&'i mut T> for CollectorMut<'a, T>
where
    T: Copy,
{
    #[inline]
    fn collect(&mut self, &mut item: &'i mut T) -> ControlFlow<()> {
        self.0.push(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = &'i mut T>) -> ControlFlow<()> {
        self.0.extend(items.into_iter().map(|&mut item| item));
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = &'i mut T>) -> Self::Output {
        self.0.extend(items.into_iter().map(|&mut item| item));
        self.0
    }
}

impl<T> Default for IntoCollector<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// # Examples
///
/// ```
/// use komadori::prelude::*;
///
/// let matrix = [vec![1, 2], vec![3, 4, 5], vec![6]];
///
/// let array = matrix
///     .into_iter()
///     .feed_into(Vec::new().into_concat());
///
/// assert_eq!(array, [1, 2, 3, 4, 5, 6]);
/// ```
impl<T> Concat for Vec<T> {}

/// See [`std::slice::Concat`] for why this trait bound is used.
impl<S, T> ConcatItem<Vec<T>> for S
where
    S: Borrow<[T]>,
    T: Clone,
{
}

impl<T> ConcatSealed for Vec<T> {}

impl<S, T> ConcatItemSealed<Vec<T>> for S
where
    S: Borrow<[T]>,
    T: Clone,
{
    #[inline]
    fn push_to(&mut self, owned_slice: &mut Vec<T>) {
        owned_slice.extend_from_slice((*self).borrow());
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::prelude::*;
    use crate::test_utils::{
        BasicCollectorTester, CollectorTestParts, CollectorTester, CollectorTesterExt, PredError,
        none_iter_for_fuse_test,
    };

    proptest! {
        #[test]
        fn all_collect_methods_into(
            starting_nums in propvec(any::<i32>(), ..5),
            nums in propvec(any::<i32>(), ..5),
        ) {
            all_collect_methods_into_impl(starting_nums, nums)?;
        }
    }

    fn all_collect_methods_into_impl(starting_nums: Vec<i32>, nums: Vec<i32>) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().cloned(),
            collector_factory: || starting_nums.clone().into_collector(),
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                let mut starting_nums = starting_nums.clone();

                // Quite redundant, but we also wanna check for the equivalence to `Iterator::collect()`.
                if starting_nums.is_empty() {
                    starting_nums = iter.collect();
                } else {
                    starting_nums.extend(iter);
                }

                if output != starting_nums {
                    Err(PredError::IncorrectOutput)
                } else if remaining.count() != 0 {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            },
        }
        .test_collector()
    }

    proptest! {
        #[test]
        fn all_collect_methods_mut(
            starting_nums in propvec(any::<i32>(), ..5),
            nums in propvec(any::<i32>(), ..5),
        ) {
            all_collect_methods_mut_impl(starting_nums, nums)?;
        }
    }

    fn all_collect_methods_mut_impl(starting_nums: Vec<i32>, nums: Vec<i32>) -> TestCaseResult {
        CollectorMutTester::new(starting_nums, nums).test_collector()
    }

    struct CollectorMutTester {
        starting_nums: Vec<i32>,
        collector_base: Vec<i32>,
        nums: Vec<i32>,
        expected_output: Vec<i32>,
    }

    impl CollectorMutTester {
        fn new(starting_nums: Vec<i32>, nums: Vec<i32>) -> Self {
            let mut expected_output = starting_nums.clone();
            expected_output.extend_from_slice(&nums);

            CollectorMutTester {
                starting_nums,
                collector_base: vec![],
                nums,
                expected_output,
            }
        }
    }

    impl CollectorTester for CollectorMutTester {
        type Item<'a> = i32;
        type Output<'a> = &'a mut Vec<i32>;

        fn collector_test_parts<'a>(
            &'a mut self,
        ) -> CollectorTestParts<
            impl Iterator<Item = Self::Item<'a>>,
            impl Collector<Self::Item<'a>, Output = Self::Output<'a>>,
            impl FnMut(
                Self::Output<'a>,
                &mut dyn Iterator<Item = Self::Item<'a>>,
            ) -> Result<(), PredError>,
            impl Iterator<Item = Self::Item<'a>>,
        > {
            // Don't forget to reset the collector.
            self.collector_base.clone_from(&self.starting_nums);

            // It has to be here because of "lifetime may not live long enough."
            let output_pred = |output: Self::Output<'_>, iter: &mut dyn Iterator<Item = _>| {
                if *output != self.expected_output {
                    Err(PredError::IncorrectOutput)
                } else if iter.count() > 0 {
                    Err(PredError::IncorrectIterConsumption)
                } else {
                    Ok(())
                }
            };

            CollectorTestParts {
                iter: self.nums.iter().cloned(),
                collector: self.collector_base.collector_mut(),
                should_break: false,
                pred: output_pred,
                iter_for_fuse_test: none_iter_for_fuse_test(),
            }
        }
    }
}
