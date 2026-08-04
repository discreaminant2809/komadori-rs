use core::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase, finish_boxed_impl};

/// A collector that calls a closure on each item before collecting.
///
/// This `struct` is created by [`CollectorBase::inspect()`]. See its documentation for more.
#[derive(Clone)]
pub struct Inspect<C, F> {
    collector: C,
    f: F,
}

impl<C, F> Inspect<C, F> {
    pub(in crate::collector) fn new(collector: C, f: F) -> Self {
        Self { collector, f }
    }
}

impl<C, F> CollectorBase for Inspect<C, F>
where
    C: CollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    finish_boxed_impl!();

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.collector.reserve(additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.collector.max_afford(request)
    }
}

impl<C, T, F> Collector<T> for Inspect<C, F>
where
    C: Collector<T>,
    F: FnMut(&T),
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        (self.f)(&item);
        self.collector.collect(item)
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
        (self.f)(&item);
        // SAFETY: The caller has reserved at least 1 item.
        unsafe { self.collector.assume_reserved_collect(item) }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        // `inspect()` does not implement `TrustedLen`, which is funny
        // but could hurt performance if it matters (e.g. collecting to `Vec`).
        #[allow(clippy::manual_inspect)]
        self.collector.collect_many(items.into_iter().map(|item| {
            (self.f)(&item);
            item
        }))
    }

    #[inline]
    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        // `inspect()` does not implement `TrustedLen`, which is funny
        // but could hurt performance if it matters (e.g. collecting to `Vec`).
        #[allow(clippy::manual_inspect)]
        self.collector
            .collect_then_finish(items.into_iter().map(move |item| {
                (self.f)(&item);
                item
            }))
    }
}

impl<C: Debug, F> Debug for Inspect<C, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Inspect")
            .field("collector", &self.collector)
            .field("f", &core::any::type_name::<F>())
            .finish()
    }
}

#[cfg(all(test, feature = "std"))]
mod proptests {
    use core::cell::Cell;

    use crate::test_utils::prelude::*;

    use super::super::take_collector_model;

    collector_test!(adapter {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let n = ..=5_usize;
        },
        iter: nums.iter().map(|&num| Cell::new(num)),
        collector: vec![].into_collector().take(n).inspect(f),
        expected_f: |iter, count| {
            let res: Vec<_> = iter.inspect(f).take(n).collect();
            (res, count >= n)
        },
        output_pred: PartialEq::eq,
        model: take_collector_model(n),
    });

    fn f(num: &Cell<i32>) {
        let old = num.get();
        num.set(old.wrapping_add(i32::MAX));
    }
}
