use std::fmt::Debug;

use crate::{
    collector::{MapBase, UnindexedParallelCollector},
    ops::ParTrying,
};

/// A parallel collector that sets the [`Output`] to [`Err(e)`](Err) when
/// an [`Err(e)`](Err) item is encountered *anywhere*,
/// else the underlying collector collects the `item` inside
/// [`Ok(item)`](Ok).
///
/// This `struct` is created by [`UnindexedParallelCollectorBase::trying_results()`].
/// See its documentation for more.
///
/// [`UnindexedParallelCollectorBase::trying_results()`]: crate::collector::UnindexedParallelCollectorBase::trying_results
/// [`Output`]: crate::collector::ParallelCollectorBase::Output
// `MapBase` already uniquifies their serial collector types,
// and `AddPhantomData` guarantees uniqueness to `Map` and `MapWith`.
pub type TryingResults<C, E> =
    MapBase<ParTrying<Result<C, E>>, closure::TryingResultsAddPhantomData>;

impl<C, E> TryingResults<C, E> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        MapBase::new_base(
            ParTrying::new(Ok(collector)),
            closure::TryingResultsAddPhantomData,
        )
    }
}

fn _check_impl_par_collector<C, T, E>(collector: TryingResults<C, E>)
where
    C: UnindexedParallelCollector<T>,
    E: Send,
{
    fn assert<T>(_: impl UnindexedParallelCollector<T>) {}
    assert::<Result<T, E>>(collector);
}

fn _check_impl_clone<C, E>(collector: TryingResults<C, E>)
where
    C: Clone,
    E: Clone,
{
    fn assert(_: impl Clone) {}
    assert(collector);
}

fn _check_impl_debug<C, E>(collector: TryingResults<C, E>)
where
    C: Debug,
    E: Debug,
{
    fn assert(_: impl Debug) {}
    assert(collector);
}

mod closure {
    use std::marker::PhantomData;

    use crate::ops::{
        CallMut, CallOnce, DefineCallMut, DefineCallOnce, ParallelFnMutBase, ParallelFnOnceBase,
    };

    // Named this way for nicer debug.
    #[derive(Debug, Clone)]
    pub struct TryingResultsAddPhantomData;

    #[expect(missing_debug_implementations)]
    pub struct Callable;

    impl<'a> DefineCallOnce<'a> for TryingResultsAddPhantomData {
        type CallOnce = Callable;
    }

    impl<'a> DefineCallMut<'a> for TryingResultsAddPhantomData {
        type CallMut = Callable;
    }

    impl ParallelFnOnceBase for TryingResultsAddPhantomData {
        fn callable_once<'a>(
            &'a mut self,
        ) -> impl FnOnce() -> <Self as DefineCallOnce<'a>>::CallOnce + Clone + Send {
            || Callable
        }
    }

    impl ParallelFnMutBase for TryingResultsAddPhantomData {
        fn callable_mut<'a>(
            &'a mut self,
        ) -> impl FnOnce() -> <Self as DefineCallMut<'a>>::CallMut + Clone + Send {
            || Callable
        }
    }

    impl<T, E> CallOnce<(Result<T, E>,)> for Callable {
        type Output = (Result<T, E>, PhantomData<T>);

        #[inline]
        fn call_once(self, (res,): (Result<T, E>,)) -> Self::Output {
            (res, PhantomData)
        }
    }

    impl<T, E> CallMut<(Result<T, E>,)> for Callable {
        #[inline]
        fn call_mut(&mut self, (res,): (Result<T, E>,)) -> Self::Output {
            (res, PhantomData)
        }
    }
}

#[cfg(test)]
mod proptests {
    use komadori::{collector::partition, iter::IteratorExt};

    use crate::test_utils::prelude::*;

    par_collector_test!(indexed {
        iter_data: {
            // Different types so that we can know which one is the error type.
            let mut nums = propvec(any::<Result<i32, u32>>(), ..=5_usize);
        },
        other_data: {
            let mut n = ..=5_usize;
        },
        iter: nums.par_iter().cloned(),
        collector: vec![].into_par_collector().take(n).trying_results(),
        starting_ma_f: |request: usize| request.min(n),
        expected_f: |iter, _, request| {
            let (errs, oks) = iter.map(Result::into).feed_into(partition(vec![], vec![]));
            let status = if oks.len() >= n || !errs.is_empty() {
                Break(())
            } else {
                Continue(((), request.min(n - oks.len())))
            };

            ((oks, errs), status)
        },
        output_pred: |actual, (oks, errs)| match actual {
            Ok(actual) => actual.len() == oks.len().min(n) && is_subsequence(actual, oks),
            Err(actual) => errs.contains(actual),
        },
        state_pred: state_is_irrelevant(),
    });

    unindexed_par_collector_test!(unindexed {
        iter_data: {
            // Different types so that we can know which one is the error type.
            let mut nums = propvec(any::<Result<i32, u32>>(), ..=5_usize);
        },
        other_data: {
            let mut n = ..=5_usize;
        },
        iter: nums.par_iter().cloned(),
        collector: vec![].into_par_collector().take(n).trying_results(),
        starting_ma_f: |request: usize| request.min(n),
        expected_f: |iter, _, request| {
            let (errs, oks) = iter.map(Result::into).feed_into(partition(vec![], vec![]));
            let status = if oks.len() >= n || !errs.is_empty() {
                Break(())
            } else {
                Continue(((), request.min(n - oks.len())))
            };

            ((oks, errs), status)
        },
        output_pred: |actual, (oks, errs)| match actual {
            Ok(actual) => actual.len() == oks.len().min(n) && is_subsequence(actual, oks),
            Err(actual) => errs.contains(actual),
        },
        state_pred: state_is_irrelevant(),
    });
}
