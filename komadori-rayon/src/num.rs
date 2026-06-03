//! Numeric-related collectors.
//!
//! This module provides parallel collectors to do operations on numeric types
//! in the standard library.
//!
//! This module corresponds to [`std::num`].

use std::{num::Wrapping, ops::ControlFlow};

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
    ops,
};

/// A parallel collector that adds every collected number.
/// Its [`Output`](ParallelCollectorBase::Output) is the type
/// that created this parallel collector.
///
/// Its [`Default`] implementation provides the "additive identity"
/// of `Num`.
///
/// This `struct` is created by [`[number].into_par_sum()`](ops::IntoParSum),
/// where `[number]`'s type is, currently, all integers and floating point numbers,
/// as well as [`Wrapping`].
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::prelude::*;
///
/// let sum = (1..=100)
///     .into_par_iter()
///     .feed_into(0.into_par_sum());
///
/// assert_eq!(sum, 5050);
/// ```
#[derive(Debug, Clone)]
pub struct IntoParSum<Num>(Num);

/// A parallel collector that multiplies every collected number.
/// Its [`Output`](ParallelCollectorBase::Output) is the type
/// that created this parallel collector.
///
/// Its [`Default`] implementation provides the "multiplicative identity"
/// of `Num`, such as `1`.
///
/// This `struct` is created by [`[number].into_par_product()`](ops::IntoParProduct),
/// where `[number]`'s type is, currently, all integers and floating point numbers,
/// as well as [`Wrapping`].
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::prelude::*;
///
/// let product = (1..=10)
///     .into_par_iter()
///     // Be careful: 0 will nullify every number we multiply!
///     .feed_into(1.into_par_product());
///
/// assert_eq!(product, 3628800);
/// ```
#[derive(Debug, Clone)]
pub struct IntoParProduct<Num>(Num);

macro_rules! prim_sum_impl {
    ($PrimTy:ty, $identity:expr) => {
        impl ops::IntoParSum for $PrimTy {
            type IntoParSum = IntoParSum<$PrimTy>;

            #[inline]
            fn into_par_sum(self) -> Self::IntoParSum {
                assert_unindexed_par_collector::<_, $PrimTy>(assert_unindexed_par_collector::<_, &$PrimTy>(
                    assert_unindexed_par_collector::<_, &mut $PrimTy>(IntoParSum(self)),
                ))
            }
        }

        impl Default for IntoParSum<$PrimTy> {
            #[inline]
            fn default() -> Self {
                assert_unindexed_par_collector::<_, $PrimTy>(assert_unindexed_par_collector::<_, &$PrimTy>(
                    assert_unindexed_par_collector::<_, &mut $PrimTy>(IntoParSum($identity)),
                ))
            }
        }

        impl<'this> DefineSerial<'this> for IntoParSum<$PrimTy> {
            type Serial = unique::Serial<'this, Self, sum::Serial<$PrimTy>>;
        }

        impl<'this> DefineUnindexedSerial<'this> for IntoParSum<$PrimTy> {
            type UnindexedSerial = unique_unindexed::Serial<'this, Self, sum::Serial<$PrimTy>>;
        }

        impl ParallelCollectorBase for IntoParSum<$PrimTy> {
            type Output = $PrimTy;

            #[inline]
            fn finish(self) -> Self::Output {
                self.0
            }

            fn parts<'a>(
                &'a mut self,
                len: usize,
            ) -> (
                usize,
                impl Consumer<
                    IntoCollector = <Self as DefineSerial<'a>>::Serial,
                    Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
                >,
                impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
            ) {
                unique::uniquify((len, sum::Consumer::new(), |count| {
                    self.0 += count;
                    ControlFlow::Continue(())
                }))
            }
        }

        impl UnindexedParallelCollectorBase for IntoParSum<$PrimTy> {
            fn parts_unindexed<'a>(
                &'a mut self,
            ) -> (
                impl UnindexedConsumer<
                    IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
                    Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
                >,
                impl FnOnce(
                    <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
                ) -> ControlFlow<()>,
            ) {
                unique_unindexed::uniquify((sum::Consumer::new(), |count| {
                    self.0 += count;
                    ControlFlow::Continue(())
                }))
            }
        }
    };
}

macro_rules! prim_product_impl {
    ($PrimTy:ty, $identity:expr) => {
        impl ops::IntoParProduct for $PrimTy {
            type IntoParProduct = IntoParProduct<$PrimTy>;

            #[inline]
            fn into_par_product(self) -> Self::IntoParProduct {
                assert_unindexed_par_collector::<_, $PrimTy>(assert_unindexed_par_collector::<_, &$PrimTy>(
                    assert_unindexed_par_collector::<_, &mut $PrimTy>(IntoParProduct(self)),
                ))
            }
        }

        impl Default for IntoParProduct<$PrimTy> {
            #[inline]
            fn default() -> Self {
                assert_unindexed_par_collector::<_, $PrimTy>(assert_unindexed_par_collector::<_, &$PrimTy>(
                    assert_unindexed_par_collector::<_, &mut $PrimTy>(IntoParProduct($identity)),
                ))
            }
        }

        impl<'this> DefineSerial<'this> for IntoParProduct<$PrimTy> {
            type Serial = unique::Serial<'this, Self, product::Serial<$PrimTy>>;
        }

        impl<'this> DefineUnindexedSerial<'this> for IntoParProduct<$PrimTy> {
            type UnindexedSerial = unique_unindexed::Serial<'this, Self, product::Serial<$PrimTy>>;
        }

        impl ParallelCollectorBase for IntoParProduct<$PrimTy> {
            type Output = $PrimTy;

            #[inline]
            fn finish(self) -> Self::Output {
                self.0
            }

            fn parts<'a>(
                &'a mut self,
                len: usize,
            ) -> (
                usize,
                impl Consumer<
                    IntoCollector = <Self as DefineSerial<'a>>::Serial,
                    Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
                >,
                impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
            ) {
                unique::uniquify((len, product::Consumer::new(), |count| {
                    self.0 *= count;
                    ControlFlow::Continue(())
                }))
            }
        }

        impl UnindexedParallelCollectorBase for IntoParProduct<$PrimTy> {
            fn parts_unindexed<'a>(
                &'a mut self,
            ) -> (
                impl UnindexedConsumer<
                    IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
                    Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
                >,
                impl FnOnce(
                    <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
                ) -> ControlFlow<()>,
            ) {
                unique_unindexed::uniquify((product::Consumer::new(), |count| {
                    self.0 *= count;
                    ControlFlow::Continue(())
                }))
            }
        }
    };
}

macro_rules! int_impls {
    ($($int_ty:ty)*) => {$(
        prim_sum_impl!($int_ty, 0);
        prim_product_impl!($int_ty, 1);

        prim_sum_impl!(Wrapping<$int_ty>, Wrapping(0));
        prim_product_impl!(Wrapping<$int_ty>, Wrapping(1));
    )*};
}
int_impls!(usize u8 u16 u32 u64 u128 isize i8 i16 i32 i64 i128);

macro_rules! float_impls {
    ($($float_ty:ty)*) => {$(
        // The "additive identity" of floating point number is -0.0, not 0.0.
        // See https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.sum.
        prim_sum_impl!($float_ty, -0.0);
        prim_product_impl!($float_ty, 1.0);
    )*};
}
float_impls!(f32 f64);

#[allow(missing_debug_implementations)]
mod sum {
    use std::{marker::PhantomData, ops::AddAssign};

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumer};

    pub struct Consumer<Num>(PhantomData<Num>);

    pub struct Combiner(());

    pub type Serial<Num> = komadori::num::IntoSum<Num>;

    impl<Num> Consumer<Num> {
        #[inline]
        pub(super) fn new() -> Self {
            Self(PhantomData)
        }
    }

    impl<Num> IntoCollectorBase for Consumer<Num>
    where
        Serial<Num>: Default + CollectorBase<Output = Num>,
    {
        type Output = Num;

        type IntoCollector = Serial<Num>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial::default()
        }
    }

    impl<Num> plumbing::Consumer for Consumer<Num>
    where
        Serial<Num>: Default + CollectorBase<Output = Num>,
        Num: AddAssign + Send,
    {
        type Combiner = Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }
    }

    impl<Num> UnindexedConsumer for Consumer<Num>
    where
        Serial<Num>: Default + CollectorBase<Output = Num>,
        Num: AddAssign + Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self::new()
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            Combiner(())
        }
    }

    impl<Num> plumbing::Combiner<Num> for Combiner
    where
        Num: AddAssign,
    {
        #[inline]
        fn combine(self, left: &mut Num, right: Num) {
            *left += right;
        }
    }
}

#[allow(missing_debug_implementations)]
mod product {

    use std::{marker::PhantomData, ops::MulAssign};

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumer};

    pub struct Consumer<Num>(PhantomData<Num>);

    pub struct Combiner(());

    pub type Serial<Num> = komadori::num::IntoProduct<Num>;

    impl<Num> Consumer<Num> {
        #[inline]
        pub(super) fn new() -> Self {
            Self(PhantomData)
        }
    }

    impl<Num> IntoCollectorBase for Consumer<Num>
    where
        Serial<Num>: Default + CollectorBase<Output = Num>,
    {
        type Output = Num;

        type IntoCollector = Serial<Num>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial::default()
        }
    }

    impl<Num> plumbing::Consumer for Consumer<Num>
    where
        Serial<Num>: Default + CollectorBase<Output = Num>,
        Num: MulAssign + Send,
    {
        type Combiner = Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }
    }

    impl<Num> UnindexedConsumer for Consumer<Num>
    where
        Serial<Num>: Default + CollectorBase<Output = Num>,
        Num: MulAssign + Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self::new()
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            Combiner(())
        }
    }

    impl<Num> plumbing::Combiner<Num> for Combiner
    where
        Num: MulAssign,
    {
        #[inline]
        fn combine(self, left: &mut Num, right: Num) {
            *left *= right;
        }
    }
}

#[cfg(test)]
mod proptests {
    use crate::{
        ops::{IntoParProduct, IntoParSum},
        test_utils::prelude::*,
    };

    proptest! {
        #[test]
        fn sum_indexed(
            starting_num in -10_000..=10_000,
            (split_decision, nums) in propvec(-10_000..=10_000, ..5)
                .prop_flat_map(|nums| {
                    (IndexedSplitStrategy::new(nums.len(), DEFAULT_MAX_DEPTH), Just(nums))
                }),
            pool in CoroutinePool::prop(),
        ) {
            sum_indexed_impl(pool, split_decision, starting_num, nums)?;
        }
    }

    proptest! {
        #[test]
        fn sum_unindexed(
            starting_num in -10_000..=10_000,
            nums in propvec(-10_000..=10_000, ..5),
            pool in CoroutinePool::prop(),
            split_decision in UnindexedSplitStrategy::new(DEFAULT_MAX_DEPTH),
        ) {
            sum_unindexed_impl(pool, split_decision, starting_num, nums)?;
        }
    }

    proptest! {
        #[test]
        fn product_indexed(
            starting_num in -10..=10,
            (split_decision, nums) in propvec(-10..=10, ..5)
                .prop_flat_map(|nums| {
                    (IndexedSplitStrategy::new(nums.len(), DEFAULT_MAX_DEPTH), Just(nums))
                }),
            pool in CoroutinePool::prop(),
        ) {
            product_indexed_impl(pool, split_decision, starting_num, nums)?;
        }
    }

    proptest! {
        #[test]
        fn product_unindexed(
            starting_num in -10..=10,
            nums in propvec(-10..=10, ..5),
            pool in CoroutinePool::prop(),
            split_decision in UnindexedSplitStrategy::new(DEFAULT_MAX_DEPTH),
        ) {
            product_unindexed_impl(pool, split_decision, starting_num, nums)?;
        }
    }

    fn sum_indexed_impl(
        mut pool: CoroutinePool,
        split_decision: IndexedSplitDecision,
        starting_num: i32,
        nums: Vec<i32>,
    ) -> TestCaseResult {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: || starting_num.into_par_sum(),
            should_break_pred: |_| false,
            pred: |_, output| PredError::assert_eq(output, starting_num + nums.iter().sum::<i32>()),
        }
        .test_par_collector(&mut pool, split_decision)
    }

    fn sum_unindexed_impl(
        mut pool: CoroutinePool,
        split_decision: UnindexedSplitDecision,
        starting_num: i32,
        nums: Vec<i32>,
    ) -> TestCaseResult {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: || starting_num.into_par_sum(),
            should_break_pred: |_| false,
            pred: |_, output| PredError::assert_eq(output, starting_num + nums.iter().sum::<i32>()),
        }
        .test_unindexed_par_collector(&mut pool, split_decision)
    }

    fn product_indexed_impl(
        mut pool: CoroutinePool,
        split_decision: IndexedSplitDecision,
        starting_num: i32,
        nums: Vec<i32>,
    ) -> TestCaseResult {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: || starting_num.into_par_product(),
            should_break_pred: |_| false,
            pred: |_, output| PredError::assert_eq(output, starting_num * nums.iter().product::<i32>()),
        }
        .test_par_collector(&mut pool, split_decision)
    }

    fn product_unindexed_impl(
        mut pool: CoroutinePool,
        split_decision: UnindexedSplitDecision,
        starting_num: i32,
        nums: Vec<i32>,
    ) -> TestCaseResult {
        BasicParallelCollectorTester {
            iter_factory: || nums.par_iter().cloned(),
            collector_factory: || starting_num.into_par_product(),
            should_break_pred: |_| false,
            pred: |_, output| PredError::assert_eq(output, starting_num * nums.iter().product::<i32>()),
        }
        .test_unindexed_par_collector(&mut pool, split_decision)
    }
}
