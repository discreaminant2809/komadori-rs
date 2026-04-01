//! Numeric-related collectors.
//!
//! This module provides parallel collectors to do operations on numeric types
//! in the standard library.
//!
//! This module corresponds to [`std::num`].

use std::{num::Wrapping, ops::ControlFlow};

use komadori::prelude::*;

use crate::collector::{
    ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
};
use crate::ops;

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
                assert_unindexed_par_collector::<_, $PrimTy>(
                assert_unindexed_par_collector::<_, &$PrimTy>(
                assert_unindexed_par_collector::<_, &mut $PrimTy>(
                    IntoParSum(self)
                )))
            }
        }

        impl Default for IntoParSum<$PrimTy> {
            #[inline]
            fn default() -> Self {
                assert_unindexed_par_collector::<_, $PrimTy>(
                assert_unindexed_par_collector::<_, &$PrimTy>(
                assert_unindexed_par_collector::<_, &mut $PrimTy>(
                    IntoParSum($identity)
                )))
            }
        }

        impl<'this> DefineConsumer<'this> for IntoParSum<$PrimTy> {
            type Consumer = add_consumer::Consumer<$PrimTy>;
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
                <Self as DefineConsumer<'a>>::Consumer,
                impl FnOnce(
                    <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
                ) -> std::ops::ControlFlow<()>,
            ) {
                let (consumer, commit) = self.parts_unindexed();
                (len, consumer, commit)
            }
        }

        impl<'this> DefineUnindexedConsumer<'this> for IntoParSum<$PrimTy> {
            type UnindexedConsumer = add_consumer::Consumer<$PrimTy>;
        }

        impl UnindexedParallelCollectorBase for IntoParSum<$PrimTy> {
            fn parts_unindexed<'a>(
                &'a mut self,
            ) -> (
                <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
                impl FnOnce(
                    <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
                ) -> ControlFlow<()>,
            ) {
                (add_consumer::Consumer::new(), |count| {
                    self.0 += count;
                    ControlFlow::Continue(())
                })
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
                assert_unindexed_par_collector::<_, $PrimTy>(
                assert_unindexed_par_collector::<_, &$PrimTy>(
                assert_unindexed_par_collector::<_, &mut $PrimTy>(
                    IntoParProduct(self)
                )))
            }
        }

        impl Default for IntoParProduct<$PrimTy> {
            #[inline]
            fn default() -> Self {
                assert_unindexed_par_collector::<_, $PrimTy>(
                assert_unindexed_par_collector::<_, &$PrimTy>(
                assert_unindexed_par_collector::<_, &mut $PrimTy>(
                    IntoParProduct($identity)
                )))
            }
        }

        impl<'this> DefineConsumer<'this> for IntoParProduct<$PrimTy> {
            type Consumer = mul_consumer::Consumer<$PrimTy>;
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
                <Self as DefineConsumer<'a>>::Consumer,
                impl FnOnce(
                    <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
                ) -> std::ops::ControlFlow<()>,
            ) {
                let (consumer, commit) = self.parts_unindexed();
                (len, consumer, commit)
            }
        }

        impl<'this> DefineUnindexedConsumer<'this> for IntoParProduct<$PrimTy> {
            type UnindexedConsumer = mul_consumer::Consumer<$PrimTy>;
        }

        impl UnindexedParallelCollectorBase for IntoParProduct<$PrimTy> {
            fn parts_unindexed<'a>(
                &'a mut self,
            ) -> (
                <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
                impl FnOnce(
                    <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
                ) -> ControlFlow<()>,
            ) {
                (mul_consumer::Consumer::new(), |count| {
                    self.0 *= count;
                    ControlFlow::Continue(())
                })
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
mod add_consumer {
    use std::marker::PhantomData;

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumerBase};

    pub struct Consumer<Num>(PhantomData<Num>);

    pub struct Combiner(());

    impl<Num> Consumer<Num> {
        #[inline]
        pub(super) fn new() -> Self {
            Self(PhantomData)
        }
    }

    macro_rules! prim_impl {
        ($PrimTy:ty) => {
            impl IntoCollectorBase for Consumer<$PrimTy> {
                type Output = $PrimTy;

                type IntoCollector = komadori::num::Adding<$PrimTy>;

                #[inline]
                fn into_collector(self) -> Self::IntoCollector {
                    <$PrimTy>::adding()
                }
            }

            impl plumbing::ConsumerBase for Consumer<$PrimTy> {
                type Combiner = Combiner;

                #[inline]
                fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
                    (self.split_off_left(), self.to_combiner())
                }
            }

            impl UnindexedConsumerBase for Consumer<$PrimTy> {
                #[inline]
                fn split_off_left(&self) -> Self {
                    Self::new()
                }

                #[inline]
                fn to_combiner(&self) -> Self::Combiner {
                    Combiner(())
                }
            }

            impl plumbing::Combiner<$PrimTy> for Combiner {
                #[inline]
                fn combine(self, left: &mut $PrimTy, right: $PrimTy) {
                    *left += right;
                }
            }
        };
    }

    macro_rules! int_impls {
        ($($IntTy:ty)*) => {$(
            prim_impl!($IntTy);
            prim_impl!(::std::num::Wrapping<$IntTy>);
        )*};
    }

    int_impls!(usize u8 u16 u32 u64 u128 isize i8 i16 i32 i64 i128);
    prim_impl!(f32);
    prim_impl!(f64);
}

#[allow(missing_debug_implementations)]
mod mul_consumer {
    use std::marker::PhantomData;

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumerBase};

    pub struct Consumer<Num>(PhantomData<Num>);

    pub struct Combiner(());

    impl<Num> Consumer<Num> {
        #[inline]
        pub(super) fn new() -> Self {
            Self(PhantomData)
        }
    }

    macro_rules! prim_impl {
        ($PrimTy:ty) => {
            impl IntoCollectorBase for Consumer<$PrimTy> {
                type Output = $PrimTy;

                type IntoCollector = komadori::num::Muling<$PrimTy>;

                #[inline]
                fn into_collector(self) -> Self::IntoCollector {
                    <$PrimTy>::muling()
                }
            }

            impl plumbing::ConsumerBase for Consumer<$PrimTy> {
                type Combiner = Combiner;

                #[inline]
                fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
                    (self.split_off_left(), self.to_combiner())
                }
            }

            impl UnindexedConsumerBase for Consumer<$PrimTy> {
                #[inline]
                fn split_off_left(&self) -> Self {
                    Self::new()
                }

                #[inline]
                fn to_combiner(&self) -> Self::Combiner {
                    Combiner(())
                }
            }

            impl plumbing::Combiner<$PrimTy> for Combiner {
                #[inline]
                fn combine(self, left: &mut $PrimTy, right: $PrimTy) {
                    *left *= right;
                }
            }
        };
    }

    macro_rules! int_impls {
        ($($IntTy:ty)*) => {$(
            prim_impl!($IntTy);
            prim_impl!(::std::num::Wrapping<$IntTy>);
        )*};
    }

    int_impls!(usize u8 u16 u32 u64 u128 isize i8 i16 i32 i64 i128);
    prim_impl!(f32);
    prim_impl!(f64);
}
