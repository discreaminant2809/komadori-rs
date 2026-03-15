//! Numeric-related collectors.
//!
//! This module provides [`Adding`](crate::ops::Adding) and [`Muling`](crate::ops::Muling)
//! collectors for numeric types in the standard library.
//!
//! This module corresponds to [`std::num`].

use std::{num::Wrapping, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, assert_collector};

/// A collector that adds every collected number.
/// Its [`Output`](CollectorBase::Output) is the type
/// that created this collector.
///
/// This `struct` is created by `<Num>::adding()`, where `Num`
/// is, currently, all integers and floating point numbers,
/// as well as [`Wrapping`].
///
/// # Examples
///
/// ```
/// use komadori::prelude::*;
///
/// let mut sum = i32::adding();
///
/// assert!(sum.collect(1).is_continue());
/// assert!(sum.collect(&2).is_continue());
/// assert!(sum.collect(&mut 3).is_continue());
///
/// assert_eq!(sum.finish(), 6);
/// ```
#[derive(Debug, Clone)]
pub struct Adding<Num>(Num);

/// A collector that adds every collected number.
/// Its [`Output`](CollectorBase::Output) is the type
/// that created this collector.
///
/// This `struct` is created by `<Num>::muling()`, where `Num`
/// is, currently, all integers and floating point numbers,
/// as well as [`Wrapping`].
///
/// # Examples
///
/// ```
/// use komadori::prelude::*;
///
/// let mut product = i32::muling();
///
/// assert!(product.collect(-1).is_continue());
/// assert!(product.collect(&2).is_continue());
/// assert!(product.collect(&mut 3).is_continue());
///
/// assert_eq!(product.finish(), -6);
/// ```
#[derive(Debug, Clone)]
pub struct Muling<Num>(Num);

macro_rules! prim_adding_impl {
    ($pri_ty:ty, $identity:expr) => {
        impl crate::ops::Adding for $pri_ty {
            type Output = $pri_ty;

            type Adding = Adding<$pri_ty>;

            #[inline]
            fn adding() -> Self::Adding {
                Default::default()
            }
        }

        impl Default for Adding<$pri_ty> {
            #[inline]
            fn default() -> Self {
                assert_collector::<_, $pri_ty>(Adding($identity))
            }
        }

        impl CollectorBase for Adding<$pri_ty> {
            type Output = $pri_ty;

            #[inline]
            fn finish(self) -> Self::Output {
                self.0
            }
        }

        impl Collector<$pri_ty> for Adding<$pri_ty> {
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

        impl<'a> Collector<&'a $pri_ty> for Adding<$pri_ty> {
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

        impl<'a> Collector<&'a mut $pri_ty> for Adding<$pri_ty> {
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

macro_rules! prim_muling_impl {
    ($pri_ty:ty, $identity:expr) => {
        impl crate::ops::Muling for $pri_ty {
            type Output = $pri_ty;

            type Muling = Muling<$pri_ty>;

            #[inline]
            fn muling() -> Self::Muling {
                Default::default()
            }
        }

        impl Default for Muling<$pri_ty> {
            #[inline]
            fn default() -> Self {
                assert_collector::<_, $pri_ty>(Muling($identity))
            }
        }

        impl CollectorBase for Muling<$pri_ty> {
            type Output = $pri_ty;

            #[inline]
            fn finish(self) -> Self::Output {
                self.0
            }
        }

        impl Collector<$pri_ty> for Muling<$pri_ty> {
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

        impl<'a> Collector<&'a $pri_ty> for Muling<$pri_ty> {
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

        impl<'a> Collector<&'a mut $pri_ty> for Muling<$pri_ty> {
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
            collector_factory: || i32::adding(),
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
            collector_factory: || i64::muling(),
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
