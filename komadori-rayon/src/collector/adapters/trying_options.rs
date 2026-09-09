use std::fmt::Debug;

use crate::{
    collector::{MapBase, UnindexedParallelCollector},
    ops::ParTrying,
};

/// A parallel collector that sets the [`Output`] to [`None`] when
/// a [`None`] item is encountered *anywhere*,
/// else the underlying collector collects the `item` inside
/// [`Some(item)`](Some).
///
/// This `struct` is created by [`UnindexedParallelCollectorBase::trying_options()`].
/// See its documentation for more.
///
/// [`UnindexedParallelCollectorBase::trying_options()`]: crate::collector::UnindexedParallelCollectorBase::trying_options
/// [`Output`]: crate::collector::ParallelCollectorBase::Output
// `MapBase` already uniquifies their serial collector types,
// and `TryingOptionsAddPhantomData` guarantees uniqueness to `Map` and `MapWith`.
pub type TryingOptions<C> = MapBase<ParTrying<Option<C>>, closure::TryingOptionsAddPhantomData>;

impl<C> TryingOptions<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        MapBase::new_base(
            ParTrying::new(Some(collector)),
            closure::TryingOptionsAddPhantomData,
        )
    }
}

fn _check_impl_par_collector<C, T>(collector: TryingOptions<C>)
where
    C: UnindexedParallelCollector<T>,
{
    fn assert<T>(_: impl UnindexedParallelCollector<T>) {}
    assert::<Option<T>>(collector);
}

fn _check_impl_clone<C>(collector: TryingOptions<C>)
where
    C: Clone,
{
    fn assert(_: impl Clone) {}
    assert(collector);
}

fn _check_impl_debug<C>(collector: TryingOptions<C>)
where
    C: Debug,
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
    pub struct TryingOptionsAddPhantomData;

    #[expect(missing_debug_implementations)]
    pub struct Callable;

    impl<'a> DefineCallOnce<'a> for TryingOptionsAddPhantomData {
        type CallOnce = Callable;
    }

    impl<'a> DefineCallMut<'a> for TryingOptionsAddPhantomData {
        type CallMut = Callable;
    }

    impl ParallelFnOnceBase for TryingOptionsAddPhantomData {
        fn callable_once<'a>(
            &'a mut self,
        ) -> impl FnOnce() -> <Self as DefineCallOnce<'a>>::CallOnce + Clone + Send {
            || Callable
        }
    }

    impl ParallelFnMutBase for TryingOptionsAddPhantomData {
        fn callable_mut<'a>(
            &'a mut self,
        ) -> impl FnOnce() -> <Self as DefineCallMut<'a>>::CallMut + Clone + Send {
            || Callable
        }
    }

    impl<T> CallOnce<(Option<T>,)> for Callable {
        type Output = (Option<T>, PhantomData<T>);

        #[inline]
        fn call_once(self, (opt,): (Option<T>,)) -> Self::Output {
            (opt, PhantomData)
        }
    }

    impl<T> CallMut<(Option<T>,)> for Callable {
        #[inline]
        fn call_mut(&mut self, (opt,): (Option<T>,)) -> Self::Output {
            (opt, PhantomData)
        }
    }
}

#[cfg(test)]
mod proptests {
    use komadori::{clb_mut, ops::Or, prelude::*};

    use crate::test_utils::prelude::*;

    par_collector_test!(indexed {
        iter_data: {
            let mut nums = propvec(any::<Option<i32>>(), ..=5_usize);
        },
        other_data: {
            let mut n = ..=5_usize;
        },
        iter: nums.par_iter().cloned(),
        collector: vec![].into_par_collector().take(n).trying_options(),
        starting_ma_f: |request: usize| request.min(n),
        expected_f: |iter, _, request| {
            let (any_none, items) = iter.feed_into((
                Or::new().map(clb_mut!(|item: &mut Option<i32>| -> bool {
                    item.is_none()
                })),
                vec![].into_collector().filter_map(|item| item),
            ));

            let status = if items.len() >= n || any_none {
                Break(())
            } else {
                Continue(((), request.min(n - items.len())))
            };

            ((any_none, items), status)
        },
        output_pred: |actual, &(any_none, ref items)| match actual {
            Some(actual) => actual.len() == items.len().min(n) && is_subsequence(actual, items),
            None => any_none,
        },
        state_pred: state_is_irrelevant(),
    });

    unindexed_par_collector_test!(unindexed {
        iter_data: {
            let mut nums = propvec(any::<Option<i32>>(), ..=5_usize);
        },
        other_data: {
            let mut n = ..=5_usize;
        },
        iter: nums.par_iter().cloned(),
        collector: vec![].into_par_collector().take(n).trying_options(),
        starting_ma_f: |request: usize| request.min(n),
        expected_f: |iter, _, request| {
            let (any_none, items) = iter.feed_into((
                Or::new().map(clb_mut!(|item: &mut Option<i32>| -> bool {
                    item.is_none()
                })),
                vec![].into_collector().filter_map(|item| item),
            ));

            let status = if items.len() >= n || any_none {
                Break(())
            } else {
                Continue(((), request.min(n - items.len())))
            };

            ((any_none, items), status)
        },
        output_pred: |actual, &(any_none, ref items)| match actual {
            Some(actual) => actual.len() == items.len().min(n) && is_subsequence(actual, items),
            None => any_none,
        },
        state_pred: state_is_irrelevant(),
    });
}
