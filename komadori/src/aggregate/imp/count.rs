#![allow(private_bounds)]

use std::{fmt::Debug, marker::PhantomData};

use crate::aggregate::{AggregateOp, RefAggregateOp, assert_ref_op};

/// A [`RefAggregateOp`] that counts how many items it has operated on.
///
/// The default count type is `usize`. If you want a different count type,
/// you can write `Count<_, _, [count type]>`, or specify the type in the map type.
/// Currently, all integer types in the standard library are supported.
///
/// # Panics
///
/// If the count is the maximum value of the count type, operating on further items
/// may panic or produce the wrong result.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use komadori::{
///     prelude::*,
///     aggregate::{self, GroupMap},
/// };
///
/// let mut collector = HashMap::<_, i32>::new()
///     .into_aggregate(aggregate::Count::new());
///
/// assert!(collector.collect((1, 1)).is_continue());
/// assert!(collector.collect((1, 2)).is_continue());
/// assert!(collector.collect((2, 1)).is_continue());
/// assert!(collector.collect((1, 4)).is_continue());
/// assert!(collector.collect((2, 3)).is_continue());
///
/// let counts = collector.finish();
///
/// assert_eq!(counts[&1], 3);
/// assert_eq!(counts[&2], 2);
/// ```
pub struct Count<K, T, C: SupportedCountTy = usize> {
    _maker: PhantomData<fn(&K, T, &mut C) -> C>,
}

impl<K, T, C: SupportedCountTy> Count<K, T, C> {
    /// Creates a new instance of this aggregate op.
    #[inline]
    pub const fn new() -> Self {
        assert_ref_op(Self {
            _maker: PhantomData,
        })
    }
}

impl<K, T, C: SupportedCountTy> Default for Count<K, T, C> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<K, T, C: SupportedCountTy> AggregateOp for Count<K, T, C> {
    type Key = K;

    type Value = C;

    type Item = T;

    #[inline]
    fn new_value(&mut self, _key: &Self::Key, _item: Self::Item) -> Self::Value {
        C::ONE
    }

    #[inline]
    fn modify(&mut self, value: &mut Self::Value, _item: Self::Item) {
        value.increment();
    }
}

impl<K, T, C: SupportedCountTy> RefAggregateOp for Count<K, T, C> {
    #[inline]
    fn new_value_ref(&mut self, _key: &Self::Key, _item: &mut Self::Item) -> Self::Value {
        C::ONE
    }

    #[inline]
    fn modify_ref(&mut self, value: &mut Self::Value, _item: &mut Self::Item) {
        value.increment();
    }
}

impl<K, T, C: SupportedCountTy> Clone for Count<K, T, C> {
    fn clone(&self) -> Self {
        Self {
            _maker: PhantomData,
        }
    }
}

impl<K, T, C: SupportedCountTy> Debug for Count<K, T, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Count").finish()
    }
}

trait SupportedCountTy {
    const ONE: Self;

    fn increment(&mut self);
}

macro_rules! supported_count_ty_impl {
    ($int:ty) => {
        impl SupportedCountTy for $int {
            const ONE: Self = 1 as _;

            #[inline]
            fn increment(&mut self) {
                *self += 1;
            }
        }
    };
}

macro_rules! supported_count_ty_impls {
    ($($ints:ty)*) => {
        $(supported_count_ty_impl!($ints);)*
    };
}

supported_count_ty_impls!(i8 u8 i16 u16 i32 u32 i64 u64 i128 u128 isize usize);
