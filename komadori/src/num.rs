//! Numeric-related collectors.
//!
//! This module provides [`IntoSum`](ops::IntoSum) and [`IntoProduct`](crate::ops::IntoProduct)
//! collectors for numeric types in the standard library.
//!
//! This module corresponds to [`std::num`].

#[cfg(feature = "unstable")]
use std::num::Saturating;
use std::{num::Wrapping, ops::ControlFlow};

use crate::{
    collector::{Collector, CollectorBase, assert_collector},
    ops,
};

/// A collector that adds every collected number.
/// Its [`Output`](CollectorBase::Output) is the type
/// that created this collector.
///
/// Its [`Default`] implementation provides the "additive identity"
/// of `Num`.
///
/// This `struct` is created by [`[number].into_sum()`](ops::IntoSum),
/// where `[number]`'s type is, currently, all integers and floating point numbers,
/// as well as [`Wrapping`] (for integers) and [`Saturating`] (only for unsigned integers,
/// and unstable for now).
///
/// For [`Saturating`], the collector stops when the sum is the maximum value
/// of the unsigned integer type.
///
/// # Examples
///
/// ```
/// use komadori::prelude::*;
///
/// let mut sum = 0_i32.into_sum();
///
/// assert!(sum.collect(1).is_continue());
/// assert!(sum.collect(&2).is_continue());
/// assert!(sum.collect(&mut 3).is_continue());
///
/// assert_eq!(sum.finish(), 6);
/// ```
#[derive(Debug, Clone)]
pub struct IntoSum<Num>(Num);

/// A collector that multipies every collected number.
/// Its [`Output`](CollectorBase::Output) is the type
/// that created this collector.
///
/// Its [`Default`] implementation provides the "multiplicative identity"
/// of `Num`.
///
/// This `struct` is created by [`[number].into_product()`](ops::IntoProduct),
/// where `[number]`'s type is, currently, all integers and floating point numbers,
/// as well as [`Wrapping`].
///
/// # Examples
///
/// ```
/// use komadori::prelude::*;
///
/// // Be careful: 0 will nullify every number we multiply!
/// let mut product = 1_i32.into_product();
///
/// assert!(product.collect(-1).is_continue());
/// assert!(product.collect(&2).is_continue());
/// assert!(product.collect(&mut 3).is_continue());
///
/// assert_eq!(product.finish(), -6);
/// ```
#[derive(Debug, Clone)]
pub struct IntoProduct<Num>(Num);

#[rustfmt::skip]
macro_rules! prim_adding_impl {
    ($pri_ty:ty, $identity:expr) => {
        impl ops::IntoSum for $pri_ty {
            type IntoSum = IntoSum<$pri_ty>;

            #[inline]
            fn into_sum(self) -> Self::IntoSum {
                assert_collector::<_, $pri_ty>(
                assert_collector::<_, &$pri_ty>(
                assert_collector::<_, &mut $pri_ty>(
                    IntoSum(self)
                )))
            }
        }

        impl Default for IntoSum<$pri_ty> {
            #[inline]
            fn default() -> Self {
                assert_collector::<_, $pri_ty>(
                assert_collector::<_, &$pri_ty>(
                assert_collector::<_, &mut $pri_ty>(
                    IntoSum($identity)
                )))
            }
        }

        impl CollectorBase for IntoSum<$pri_ty> {
            type Output = $pri_ty;

            #[inline]
            fn finish(self) -> Self::Output {
                self.0
            }
        }

        impl Collector<$pri_ty> for IntoSum<$pri_ty> {
            #[inline]
            fn collect(&mut self, item: $pri_ty) -> ControlFlow<()> {
                self.0 += item;
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_many(
                &mut self,
                items: impl IntoIterator<Item = $pri_ty>,
            ) -> ControlFlow<()> {
                self.0 += items.into_iter().sum::<$pri_ty>();
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_then_finish(
                mut self,
                items: impl IntoIterator<Item = $pri_ty>,
            ) -> Self::Output {
                self.0 += items.into_iter().sum::<$pri_ty>();
                self.0
            }
        }

        impl<'a> Collector<&'a $pri_ty> for IntoSum<$pri_ty> {
            #[inline]
            fn collect(&mut self, &item: &'a $pri_ty) -> ControlFlow<()> {
                self.0 += item;
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_many(
                &mut self,
                items: impl IntoIterator<Item = &'a $pri_ty>,
            ) -> ControlFlow<()> {
                self.0 += items.into_iter().sum::<$pri_ty>();
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_then_finish(
                mut self,
                items: impl IntoIterator<Item = &'a $pri_ty>,
            ) -> Self::Output {
                self.0 += items.into_iter().sum::<$pri_ty>();
                self.0
            }
        }

        impl<'a> Collector<&'a mut $pri_ty> for IntoSum<$pri_ty> {
            #[inline]
            fn collect(&mut self, &mut item: &'a mut $pri_ty) -> ControlFlow<()> {
                self.0 += item;
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_many(
                &mut self,
                items: impl IntoIterator<Item = &'a mut $pri_ty>,
            ) -> ControlFlow<()> {
                self.0 += items.into_iter().map(|&mut num| num).sum::<$pri_ty>();
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_then_finish(
                mut self,
                items: impl IntoIterator<Item = &'a mut $pri_ty>,
            ) -> Self::Output {
                self.0 += items.into_iter().map(|&mut num| num).sum::<$pri_ty>();
                self.0
            }
        }
    };
}

#[rustfmt::skip]
macro_rules! prim_muling_impl {
    ($pri_ty:ty, $identity:expr) => {
        impl ops::IntoProduct for $pri_ty {
            type IntoProduct = IntoProduct<$pri_ty>;

            #[inline]
            fn into_product(self) -> Self::IntoProduct {
                assert_collector::<_, $pri_ty>(
                assert_collector::<_, &$pri_ty>(
                assert_collector::<_, &mut $pri_ty>(
                    IntoProduct(self)
                )))
            }
        }

        impl Default for IntoProduct<$pri_ty> {
            #[inline]
            fn default() -> Self {
                assert_collector::<_, $pri_ty>(
                assert_collector::<_, &$pri_ty>(
                assert_collector::<_, &mut $pri_ty>(
                    IntoProduct($identity)
                )))
            }
        }

        impl CollectorBase for IntoProduct<$pri_ty> {
            type Output = $pri_ty;

            #[inline]
            fn finish(self) -> Self::Output {
                self.0
            }
        }

        impl Collector<$pri_ty> for IntoProduct<$pri_ty> {
            #[inline]
            fn collect(&mut self, item: $pri_ty) -> ControlFlow<()> {
                self.0 *= item;
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_many(
                &mut self,
                items: impl IntoIterator<Item = $pri_ty>,
            ) -> ControlFlow<()> {
                self.0 *= items.into_iter().product::<$pri_ty>();
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_then_finish(
                mut self,
                items: impl IntoIterator<Item = $pri_ty>,
            ) -> Self::Output {
                self.0 *= items.into_iter().product::<$pri_ty>();
                self.0
            }
        }

        impl<'a> Collector<&'a $pri_ty> for IntoProduct<$pri_ty> {
            #[inline]
            fn collect(&mut self, &item: &'a $pri_ty) -> ControlFlow<()> {
                self.0 *= item;
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_many(
                &mut self,
                items: impl IntoIterator<Item = &'a $pri_ty>,
            ) -> ControlFlow<()> {
                self.0 *= items.into_iter().product::<$pri_ty>();
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_then_finish(
                mut self,
                items: impl IntoIterator<Item = &'a $pri_ty>,
            ) -> Self::Output {
                self.0 *= items.into_iter().product::<$pri_ty>();
                self.0
            }
        }

        impl<'a> Collector<&'a mut $pri_ty> for IntoProduct<$pri_ty> {
            #[inline]
            fn collect(&mut self, &mut item: &'a mut $pri_ty) -> ControlFlow<()> {
                self.0 *= item;
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_many(
                &mut self,
                items: impl IntoIterator<Item = &'a mut $pri_ty>,
            ) -> ControlFlow<()> {
                self.0 *= items.into_iter().map(|&mut num| num).product::<$pri_ty>();
                ControlFlow::Continue(())
            }

            #[inline]
            fn collect_then_finish(
                mut self,
                items: impl IntoIterator<Item = &'a mut $pri_ty>,
            ) -> Self::Output {
                self.0 *= items.into_iter().map(|&mut num| num).product::<$pri_ty>();
                self.0
            }
        }
    };
}

macro_rules! int_impls {
    ($($int_ty:ty)*) => {$(
        prim_adding_impl!($int_ty, 0);
        prim_muling_impl!($int_ty, 1);

        prim_adding_impl!(Wrapping<$int_ty>, Wrapping(0));
        prim_muling_impl!(Wrapping<$int_ty>, Wrapping(1));
    )*};
}

int_impls!(usize u8 u16 u32 u64 u128 isize i8 i16 i32 i64 i128);

macro_rules! float_impls {
    ($($float_ty:ty)*) => {$(
        // The "additive identity" of floating point number is -0.0, not 0.0.
        // See https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.sum.
        prim_adding_impl!($float_ty, -0.0);
        prim_muling_impl!($float_ty, 1.0);
    )*};
}

float_impls!(f32 f64);

macro_rules! unsigned_saturating_add_impl {
    ($IntTy:ty) => {
        #[cfg(feature = "unstable")]
        impl ops::IntoSum for Saturating<$IntTy> {
            type IntoSum = IntoSum<Saturating<$IntTy>>;

            #[inline]
            fn into_sum(self) -> Self::IntoSum {
                assert_collector::<_, Saturating<$IntTy>>(
                    IntoSum(self),
                )
            }
        }

        #[cfg(feature = "unstable")]
        impl Default for IntoSum<Saturating<$IntTy>> {
            #[inline]
            fn default() -> Self {
                assert_collector::<_, Saturating<$IntTy>>(
                    IntoSum(Saturating(
                        0 as $IntTy,
                    )),
                )
            }
        }

        #[cfg(feature = "unstable")]
        impl CollectorBase for IntoSum<Saturating<$IntTy>> {
            type Output = Saturating<$IntTy>;

            #[inline]
            fn finish(self) -> Self::Output {
                self.0
            }

            #[inline]
            fn break_hint(&self) -> ControlFlow<()> {
                if self.0.0 < <$IntTy>::MAX {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(())
                }
            }
        }

        #[cfg(feature = "unstable")]
        impl Collector<Saturating<$IntTy>> for IntoSum<Saturating<$IntTy>> {
            #[inline]
            fn collect(&mut self, Saturating(num): Saturating<$IntTy>) -> ControlFlow<()> {
                if let Some(sum) = self.0.0.checked_add(num) {
                    self.0.0 = sum;
                    ControlFlow::Continue(())
                } else {
                    self.0.0 = <$IntTy>::MAX;
                    ControlFlow::Break(())
                }
            }
        }

        // impl<'a> Collector<&'a Saturating<$IntTy>> for IntoSum<Saturating<$IntTy>> {
        //     #[inline]
        //     fn collect(&mut self, &Saturating(num): &'a Saturating<$IntTy>) -> ControlFlow<()> {
        //         if let Some(sum) = self.0.0.checked_add(num) {
        //             self.0.0 = sum;
        //             ControlFlow::Continue(())
        //         } else {
        //             self.0.0 = <$IntTy>::MAX;
        //             ControlFlow::Break(())
        //         }
        //     }
        // }

        // impl<'a> Collector<&'a mut Saturating<$IntTy>> for IntoSum<Saturating<$IntTy>> {
        //     #[inline]
        //     fn collect(
        //         &mut self,
        //         &mut Saturating(num): &'a mut Saturating<$IntTy>,
        //     ) -> ControlFlow<()> {
        //         if let Some(sum) = self.0.0.checked_add(num) {
        //             self.0.0 = sum;
        //             ControlFlow::Continue(())
        //         } else {
        //             self.0.0 = <$IntTy>::MAX;
        //             ControlFlow::Break(())
        //         }
        //     }
        // }
    };

    ($($IntTy:ty)*) => {
        $(unsigned_saturating_add_impl!($IntTy);)*
    };
}
unsigned_saturating_add_impl!(u8 u16 u32 u64 u128 usize);

#[cfg(all(test, feature = "std"))]
mod proptests {
    use proptest::collection::vec as propvec;
    use proptest::prelude::*;
    use proptest::test_runner::TestCaseResult;

    use crate::prelude::*;
    use crate::test_utils::{BasicCollectorTester, CollectorTesterExt, PredError};

    proptest! {
        #[test]
        fn all_collect_methods_adding_int(
            nums in propvec(any::<i16>().prop_map_into::<i32>(), ..5),
        ) {
            all_collect_methods_adding_int_impl(nums)?;
        }
    }

    fn all_collect_methods_adding_int_impl(nums: Vec<i32>) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || 0.into_sum(),
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                if iter.sum::<i32>() != output {
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

    proptest! {
        #[test]
        fn all_collect_methods_muling_int(
            nums in propvec(any::<i8>().prop_map_into::<i64>(), ..5),
        ) {
            all_collect_methods_muling_int_impl(nums)?;
        }
    }

    fn all_collect_methods_muling_int_impl(nums: Vec<i64>) -> TestCaseResult {
        BasicCollectorTester {
            iter_factory: || nums.iter().copied(),
            collector_factory: || 1_i64.into_product(),
            should_break_pred: |_| false,
            pred: |iter, output, remaining| {
                if iter.product::<i64>() != output {
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
