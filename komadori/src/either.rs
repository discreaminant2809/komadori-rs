//! Re-export of [`Either`] type.
//!
//! This module is so that you can refer to [`Either`] without importing [`either`] crate.
//! However, to use all the functionalities assosciated with that type,
//! use that crate instead.
//!
//! The crate also provides collector implementations for [`Either`],
//! and extension methods related to collectors.

mod collector_impl;

pub use either::Either;

use either::map_both;

use crate::collector::{IntoCollectorBase, assert_collector_base};

/// Extension trait for [`Either`] with collector-related methods.
#[expect(private_bounds)]
pub trait EitherExt<L, R>: Sealed {
    /// Converts the inner value to a collector.
    ///
    /// The left and the right collectors must finish with
    /// the same [`Output`](crate::collector::CollectorBase::Output),
    /// and [`Either`] collects `T` when both branches can collect `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BinaryHeap;
    /// use komadori::{prelude::*, either::Either};
    ///
    /// fn bin_heap(
    ///     starting: impl Into<Vec<i32>>,
    /// ) -> impl Collector<i32, Output = BinaryHeap<i32>> {
    ///     let starting = starting.into();
    ///
    ///     if starting.is_empty() {
    ///         // If we start without any elements,
    ///         // this is more efficient (O(n)).
    ///         Either::Left(starting.into_collector().map_output(BinaryHeap::from))
    ///     } else {
    ///         Either::Right(BinaryHeap::from(starting))
    ///     }
    ///     .into_collector()
    /// }
    ///
    /// let nums = [1, 3, 2];
    ///
    /// assert_eq!(
    ///     nums.into_iter().feed_into(bin_heap([])).into_sorted_vec(),
    ///     [1, 2, 3],
    /// );
    ///
    /// assert_eq!(
    ///     nums.into_iter().feed_into(bin_heap([0, 4])).into_sorted_vec(),
    ///     [0, 1, 2, 3, 4],
    /// );
    /// ```
    fn into_collector(self) -> Either<L::IntoCollector, R::IntoCollector>
    where
        L: IntoCollectorBase,
        R: IntoCollectorBase<Output = L::Output>;

    /// Mutably borrows the inner value as a collector.
    ///
    /// The left and the right collectors must finish with
    /// the same [`Output`](crate::collector::CollectorBase::Output),
    /// and [`Either`] collects `T` when both branches can collect `T`.
    ///
    /// # Examples
    ///
    /// *Coming soon!*
    fn collector_mut<'a>(
        &'a mut self,
    ) -> Either<
        <&'a mut L as IntoCollectorBase>::IntoCollector,
        <&'a mut R as IntoCollectorBase>::IntoCollector,
    >
    where
        &'a mut L: IntoCollectorBase,
        &'a mut R: IntoCollectorBase<Output = <&'a mut L as IntoCollectorBase>::Output>;

    /// Borrows the inner value as a collector.
    ///
    /// The left and the right collectors must finish with
    /// the same [`Output`](crate::collector::CollectorBase::Output),
    /// and [`Either`] collects `T` when both branches can collect `T`.
    ///
    /// # Examples
    ///
    /// *Coming soon!*
    fn collector<'a>(
        &'a self,
    ) -> Either<
        <&'a L as IntoCollectorBase>::IntoCollector,
        <&'a R as IntoCollectorBase>::IntoCollector,
    >
    where
        &'a L: IntoCollectorBase,
        &'a R: IntoCollectorBase<Output = <&'a L as IntoCollectorBase>::Output>;
}

impl<L, R> EitherExt<L, R> for Either<L, R> {
    #[inline]
    fn into_collector(self) -> Either<L::IntoCollector, R::IntoCollector>
    where
        L: IntoCollectorBase,
        R: IntoCollectorBase<Output = L::Output>,
    {
        assert_collector_base(map_both!(self, this => this.into_collector()))
    }

    #[inline]
    fn collector_mut<'a>(
        &'a mut self,
    ) -> Either<
        <&'a mut L as IntoCollectorBase>::IntoCollector,
        <&'a mut R as IntoCollectorBase>::IntoCollector,
    >
    where
        &'a mut L: IntoCollectorBase,
        &'a mut R: IntoCollectorBase<Output = <&'a mut L as IntoCollectorBase>::Output>,
    {
        assert_collector_base(map_both!(self, this => this.into_collector()))
    }

    #[inline]
    fn collector<'a>(
        &'a self,
    ) -> Either<
        <&'a L as IntoCollectorBase>::IntoCollector,
        <&'a R as IntoCollectorBase>::IntoCollector,
    >
    where
        &'a L: IntoCollectorBase,
        &'a R: IntoCollectorBase<Output = <&'a L as IntoCollectorBase>::Output>,
    {
        assert_collector_base(map_both!(self, this => this.into_collector()))
    }
}

trait Sealed {}
impl<L, R> Sealed for Either<L, R> {}
