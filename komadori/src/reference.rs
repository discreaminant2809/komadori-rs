use core::ops::ControlFlow;

use crate::collector::{Collector, CollectorBase, finish_boxed_impl};

/// A mutable reference to a collect produce nothing.
///
/// This is useful when you *just* want to feed items to a collector without
/// finishing it.
impl<C> CollectorBase for &mut C
where
    C: CollectorBase,
{
    type Output = ();

    #[inline]
    fn finish(self) -> Self::Output {}

    finish_boxed_impl! {}

    #[inline]
    fn reserve(&mut self, additional: usize) {
        C::reserve(self, additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        C::max_afford(self, request)
    }
}

/// A mutable reference to a collect produce nothing.
///
/// This is useful when you *just* want to feed items to a collector without
/// finishing it.
impl<C, T> Collector<T> for &mut C
where
    C: Collector<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        C::collect(self, item)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: The caller has reserved for one item.
            C::assume_reserved_collect(self, item)
        }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // FIXED: specialization for unsized type.
        // We can't add `?Sized` to the bound of `C` because this method requires `Sized`.
        C::collect_many(self, items)
    }

    // The default implementation for `collect_then_finish()` is sufficient.
}

macro_rules! dyn_impl {
    ($($traits:ident)*) => {
        impl<O> CollectorBase for &mut (dyn CollectorBase<Output = O> $(+ $traits)* + '_) {
            type Output = ();

            #[inline]
            fn finish(self) -> Self::Output {}

            finish_boxed_impl! {}

            #[inline]
            fn reserve(&mut self, additional: usize) {
                <dyn CollectorBase<Output = O>>::reserve(*self, additional)
            }

            #[inline]
            fn max_afford(&self, request: usize) -> usize {
                <dyn CollectorBase<Output = O>>::max_afford(*self, request)
            }
        }

        impl<T, O> CollectorBase for &mut (dyn Collector<T, Output = O> $(+ $traits)* + '_) {
            type Output = ();

            #[inline]
            fn finish(self) -> Self::Output {}

            finish_boxed_impl! {}

            #[inline]
            fn reserve(&mut self, additional: usize) {
                <dyn Collector<T, Output = O>>::reserve(*self, additional)
            }

            #[inline]
            fn max_afford(&self, request: usize) -> usize {
                <dyn Collector<T, Output = O>>::max_afford(*self, request)
            }
        }

        impl<T, O> Collector<T> for &mut (dyn Collector<T, Output = O> $(+ $traits)* + '_) {
            #[inline]
            fn collect(&mut self, item: T) -> ControlFlow<()> {
                (**self).collect(item)
            }

            #[inline]
            unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
                unsafe {
                    // SAFETY: The caller has reserved for one item.
                    (**self).assume_reserved_collect(item)
                }
            }
        }
    }
}

dyn_impl!();
dyn_impl!(Send);
dyn_impl!(Sync);
dyn_impl!(Send Sync);

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::{collector::take_collector_model, test_utils::prelude::*};

    proptest! {
        #[test]
        fn reference_sized(
            (nums, count, seq) in propvec(any::<i32>(), ..=5)
                .prop_flat_map(|nums| {
                    let count = ::core::iter::Iterator::count(nums.iter().copied());
                    (
                        ::proptest::strategy::Just(nums),
                        ::proptest::strategy::Just(count),
                        FuzzyExecSeqStrategy::new(count)
                    )
                }
            ),
            n in ..=5_usize,
        ) {
            let mut collected_amount = 0_usize;
            let mut expected_remaining = ::core::iter::Iterator::fuse(nums.iter().copied());
            let (expected_output,is_break) = {
                let res: Vec<_> = expected_remaining
                    .by_ref()
                    .inspect(|_| collected_amount += 1)
                    .take(n)
                    .collect();
                (res, count >= n)
            };
            let mut collector = vec![].into_collector().take(n);

            let expected_output = fuzzy_execute(
                nums.iter().copied(),
                nums.iter().copied(),
                expected_remaining.count(),
                expected_output,
                |_, _| true,
                is_break.then_some(collected_amount),
                &mut collector,
                &seq,
                take_collector_model(n),
            )?;

            let actual_output = collector.finish();
            prop_assert_eq!(
                &expected_output, &actual_output,
                "mismatched output: expected {:?}, got {:?}",
                expected_output, actual_output,
            );
        }
    }

    proptest! {
        #[test]
        fn reference_dyn(
            (nums, count, seq) in propvec(any::<i32>(), ..=5)
                .prop_flat_map(|nums| {
                    let count = ::core::iter::Iterator::count(nums.iter().copied());
                    (
                        ::proptest::strategy::Just(nums),
                        ::proptest::strategy::Just(count),
                        FuzzyExecSeqStrategy::new(count)
                    )
                }
            ),
            n in ..=5_usize,
        ) {
            let mut collected_amount = 0_usize;
            let mut expected_remaining = ::core::iter::Iterator::fuse(nums.iter().copied());
            let (expected_output,is_break) = {
                let res: Vec<_> = expected_remaining
                    .by_ref()
                    .inspect(|_| collected_amount += 1)
                    .take(n)
                    .collect();
                (res, count >= n)
            };
            let mut collector = vec![].into_collector().take(n);

            let expected_output = fuzzy_execute(
                nums.iter().copied(),
                nums.iter().copied(),
                expected_remaining.count(),
                expected_output,
                |_, _| true,
                is_break.then_some(collected_amount),
                &mut collector as &mut dyn Collector<_, Output = _>,
                &seq,
                take_collector_model(n),
            )?;

            let actual_output = collector.finish();
            prop_assert_eq!(
                &expected_output, &actual_output,
                "mismatched output: expected {:?}, got {:?}",
                expected_output, actual_output,
            );
        }
    }
}
