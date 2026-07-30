//! Re-export of [`Either`] type.
//!
//! This module is so that you can refer to [`Either`] without importing [`either`] crate.
//! However, to use all the functionalities assosciated with that type,
//! use that crate instead.
//!
//! The crate also provides collector implementations for [`Either`].

use std::ops::ControlFlow;

pub use either::Either;

use either::{for_both, map_both};

use crate::collector::{Collector, CollectorBase, finish_boxed_impl};

/// [`Either`] is a collector when both branches are so.
/// Its output is [`Either`] the left's or the right's output
/// (depending on which variant is active),
/// and it collects `T` when both branches can collect `T`.
impl<L, R> CollectorBase for Either<L, R>
where
    L: CollectorBase,
    R: CollectorBase,
{
    type Output = Either<L::Output, R::Output>;

    #[inline]
    fn finish(self) -> Self::Output {
        map_both!(self, collector => collector.finish())
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

/// [`Either`] is a collector when both branches are so.
/// Its output is [`Either`] the left's or the right's output
/// (depending on which variant is active),
/// and it collects `T` when both branches can collect `T`.
impl<L, R, T> Collector<T> for Either<L, R>
where
    L: Collector<T>,
    R: Collector<T>,
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
        map_both!(self, collector => collector.collect_then_finish(items))
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
            (
                if is_left {
                    Either::Left(res)
                } else {
                    Either::Right(res)
                },
                count >= n,
            )
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });
}
