use core::{fmt::Debug, ops::ControlFlow};

use crate::{
    collector::{Collector, CollectorBase, break_hint, finish_boxed_impl},
    iter::SizeHint,
};

/// A collector that separates collected items with a separator.
///
/// This `struct` is created by [`CollectorBase::intersperse()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct Intersperse<C, S> {
    collector: C,
    first: bool,
    sep: S,
}

/// A collector that separates collected items with a separator
/// from a function.
///
/// This `struct` is created by [`CollectorBase::intersperse_with()`].
/// See its documentation for more.
#[derive(Clone)]
pub struct IntersperseWith<C, FS> {
    collector: C,
    first: bool,
    sep_f: FS,
}

impl<C, S> Intersperse<C, S> {
    pub(in crate::collector) fn new(collector: C, sep: S) -> Self {
        Self {
            collector,
            first: true,
            sep,
        }
    }
}

impl<C, FS> IntersperseWith<C, FS> {
    pub(in crate::collector) fn new(collector: C, sep_f: FS) -> Self {
        Self {
            collector,
            first: true,
            sep_f,
        }
    }
}

impl<C, FS> Debug for IntersperseWith<C, FS>
where
    C: Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("IntersperseWith")
            .field("collector", &self.collector)
            .field("first", &self.first)
            .field("sep_f", &core::any::type_name::<FS>())
            .finish()
    }
}

impl<C, S> CollectorBase for Intersperse<C, S>
where
    C: CollectorBase,
    S: Clone,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    finish_boxed_impl! {}

    #[inline]
    fn reserve(&mut self, additional: usize) {
        reserve_impl(&mut self.collector, self.first, additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        max_afford_impl(&self.collector, self.first, request)
    }
}

impl<C, S, T> Collector<T> for Intersperse<C, S>
where
    C: Collector<S> + Collector<T>,
    S: Clone,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        collect_impl(
            &mut self.collector,
            &mut self.first,
            || self.sep.clone(),
            item,
        )
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: The caller reserved for one item (also for a separator)
            assume_reserved_collect_impl(
                &mut self.collector,
                &mut self.first,
                || self.sep.clone(),
                item,
            )
        }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        collect_many_impl(
            &mut self.collector,
            &mut self.first,
            || self.sep.clone(),
            items,
        )
    }
}

impl<C, FS, S> CollectorBase for IntersperseWith<C, FS>
where
    C: CollectorBase,
    FS: FnMut() -> S,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    finish_boxed_impl! {}

    #[inline]
    fn reserve(&mut self, additional: usize) {
        reserve_impl(&mut self.collector, self.first, additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        max_afford_impl(&self.collector, self.first, request)
    }
}

impl<C, FS, S, T> Collector<T> for IntersperseWith<C, FS>
where
    C: Collector<S> + Collector<T>,
    FS: FnMut() -> S,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        collect_impl(&mut self.collector, &mut self.first, &mut self.sep_f, item)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        unsafe {
            // SAFETY: The caller reserved for one item (also for a separator)
            assume_reserved_collect_impl(
                &mut self.collector,
                &mut self.first,
                &mut self.sep_f,
                item,
            )
        }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        collect_many_impl(&mut self.collector, &mut self.first, &mut self.sep_f, items)
    }
}

#[inline]
fn true_amount(amount: usize, first: bool) -> Option<usize> {
    amount.saturating_sub(first as _).checked_add(amount)
}

#[inline]
fn reserve_impl(collector: &mut impl CollectorBase, first: bool, additional: usize) {
    let additional = true_amount(additional, first)
        .unwrap_or_else(|| panic!("reserving for {additional} items caused an overflow"));
    collector.reserve(additional);
}

#[inline]
fn max_afford_impl(collector: &impl CollectorBase, first: bool, request: usize) -> usize {
    let max_afford = collector.max_afford(true_amount(request, first).unwrap_or(usize::MAX));

    // We may have trucated if the underlying collector can afford more than usize::MAX,
    // but that's fine for now.
    match (max_afford, first) {
        (0, _) => 0,
        (max_afford, false) if max_afford % 2 == 0 => max_afford / 2,
        _ => max_afford / 2 + 1,
    }
}

#[inline]
fn collect_impl<T, S>(
    collector: &mut (impl Collector<T> + Collector<S>),
    first: &mut bool,
    sep_f: impl FnOnce() -> S,
    item: T,
) -> ControlFlow<()> {
    if !core::mem::take(first) {
        collector.collect(sep_f())?;
    }
    collector.collect(item)
}

#[inline]
unsafe fn assume_reserved_collect_impl<T, S>(
    collector: &mut (impl Collector<T> + Collector<S>),
    first: &mut bool,
    sep_f: impl FnOnce() -> S,
    item: T,
) -> ControlFlow<()> {
    if !core::mem::take(first) {
        unsafe {
            // SAFETY: We reserved for one separator.
            collector.assume_reserved_collect(sep_f())?;
        }
    }

    unsafe {
        // SAFETY: The caller reserved for at least one item.
        // This won't reach if we stop after inserting a separator beforehand.
        collector.assume_reserved_collect(item)
    }
}

#[inline]
fn collect_many_impl<T, S>(
    collector: &mut (impl Collector<T> + Collector<S>),
    first: &mut bool,
    mut sep_f: impl FnMut() -> S,
    items: impl IntoIterator<Item = T>,
) -> ControlFlow<()> {
    break_hint(collector)?;

    let mut items = items.into_iter();
    let sh = SizeHint::from_iter(&items);
    let mut lower = sh.lower();
    reserve_impl(collector, *first, lower);

    if *first {
        let Some(item) = items.next() else {
            return ControlFlow::Continue(());
        };

        *first = false;
        if lower > 0 {
            // The collector is still in the "first" state,
            // so it has reserved for `lower * 2 - 1`.
            // After this statement, the reservation is `(lower - 1) * 2`,
            // matching the `lower -= 1` here.
            lower -= 1;
            unsafe {
                // SAFETY: We've checked that the reservation > 0.
                collector.assume_reserved_collect(item)?;
            }
        } else {
            collector.collect(item)?;
        }
    }

    let assume_reserved_collect = |item| unsafe {
        // SAFETY: There are `(lower - 1) * 2` reservation left
        // for the underlying collector.
        collector.assume_reserved_collect(sep_f())?;
        collector.assume_reserved_collect(item)
    };

    if sh.exact_size().is_some() {
        items.take(lower).try_for_each(assume_reserved_collect)
    } else {
        items
            .by_ref()
            .take(lower)
            .try_for_each(assume_reserved_collect)?;

        items.try_for_each(|item| {
            collector.collect(sep_f())?;
            collector.collect(item)
        })
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use crate::test_utils::prelude::*;

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let n = ..=5_usize;
            let sep = any::<i32>();
        },
        iter: nums.iter().copied(),
        collector: vec![].into_collector().take(n).intersperse(sep),
        expected_f: |iter, _| expected_impl(iter, n, sep),
        output_pred: PartialEq::eq,
        model: collector_model(n),
    });

    collector_test!(adapter_with {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let n = ..=5_usize;
            let sep = any::<i32>();
        },
        iter: nums.iter().copied(),
        collector: vec![]
            .into_collector()
            .take(n)
            .intersperse_with(move || sep),
        expected_f: |iter, _| expected_impl(iter, n, sep),
        output_pred: PartialEq::eq,
        model: collector_model(n),
    });

    fn expected_impl(
        mut iter: impl Iterator<Item = i32>,
        mut n: usize,
        sep: i32,
    ) -> (Vec<i32>, bool) {
        let mut res = vec![];
        let mut first = true;

        // FIXME: Maybe we depend on `itertools` unconditionally in `dev-dependencies`.
        while n > 0 {
            let Some(item) = iter.next() else {
                break;
            };

            if !first {
                res.push(sep);
                n -= 1;
                if n == 0 {
                    break;
                }
            }

            first = false;
            res.push(item);
            n -= 1;
        }

        (res, n == 0)
    }

    #[derive(Clone)]
    struct State {
        n: usize,
        first: bool,
    }

    fn collector_model(
        n: usize,
    ) -> CollectorModel<State, impl FnMut(&mut State, i32), impl FnMut(&State, usize) -> usize>
    {
        CollectorModel {
            state: State { n, first: true },
            advance_f: |state: &mut State, _| {
                if state.first {
                    state.first = false;
                    state.n = state.n.saturating_sub(1);
                } else {
                    state.n = state.n.saturating_sub(2);
                }
            },
            max_afford_f: |state: &State, mut request| {
                // We'll be simulating the collection progress.
                let mut state = state.clone();

                if state.n == 0 || request == 0 {
                    return 0;
                }

                let mut collected_count = 0_usize;

                if state.first {
                    state.first = false;
                    collected_count += 1;
                    request -= 1;
                    state.n -= 1;
                }

                // Don't literally use `while`-loop because `request` can be very large.
                // The loop looks like this:
                // ```
                // while state.n > 0 && request > 0 {
                //     collected_count += 1;
                //     request -= 1;
                //     state.n = state.n.saturating_sub(2);
                // }
                // ```
                collected_count += state.n.div_ceil(2).min(request);

                collected_count
            },
        }
    }
}
