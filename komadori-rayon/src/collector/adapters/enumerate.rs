use std::ops::ControlFlow;

use crate::{
    collector::{
        ParallelCollectorBase,
        plumbing::{Consumer, DefineSerial, SerialOf, SerialOutputOf},
    },
    helpers::unique,
};

/// A parallel collector that feeds the underlying collector
/// with the position of an item alongside with the item.
///
/// This `struct` is created by [`ParallelCollectorBase::enumerate()`].
/// See its documentation for more.
#[derive(Debug, Clone)]
pub struct Enumerate<C> {
    collector: C,
    idx: usize,
}

impl<C> Enumerate<C> {
    pub(in crate::collector) fn new(collector: C) -> Self {
        Self { collector, idx: 0 }
    }
}

impl<'a, C> DefineSerial<'a> for Enumerate<C>
where
    C: DefineSerial<'a>,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<C::Serial>>;
}

impl<C> ParallelCollectorBase for Enumerate<C>
where
    C: ParallelCollectorBase,
{
    type Output = C::Output;

    #[inline]
    fn finish(self) -> Self::Output {
        self.collector.finish()
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.collector.max_afford(request)
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.collector.parts(len);
        let idx = &mut self.idx;

        unique::uniquify((consumer::indexed(consumer, *idx), move |output| {
            // If we stop early, there's no point to update the index
            // to cause an unnecessary and even incorrect panic in debug.
            commit(output)?;
            *idx += len;
            ControlFlow::Continue(())
        }))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        let (consumer, commit) = self.collector.take_parts(len);
        unique::take_uniquify((consumer::indexed(consumer, self.idx), commit))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, BasicConsumer, Consumer, OpaqueConsumer};

    pub fn indexed<C>(consumer: C, start: usize) -> OpaqueConsumer!(Serial<C::IntoCollector>)
    where
        C: Consumer,
    {
        BasicConsumer {
            state: (consumer, start),
            split_f: |(consumer, start), idx| {
                let (consumer, combiner) = consumer.split_off_left_at(idx);
                let left_start = *start;
                // The runtime is permitted to split pass we can hold,
                // so even in the debug build, we shouldn't panic!
                *start = start.wrapping_add(idx);

                ((consumer, left_start), combiner)
            },
            ma_f: |(consumer, _), request| consumer.max_afford(request),
            collector_f: |(consumer, idx)| Serial {
                collector: consumer.into_collector(),
                idx,
            },
        }
    }

    pub struct Serial<C> {
        collector: C,
        idx: usize,
    }

    impl<C> CollectorBase for Serial<C>
    where
        C: CollectorBase,
    {
        type Output = C::Output;

        #[inline]
        fn finish(self) -> Self::Output {
            self.collector.finish()
        }

        plumbing::finish_boxed_impl! {}

        #[inline]
        fn reserve(&mut self, additional: usize) {
            self.collector.reserve(additional);
        }

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            self.collector.max_afford(request)
        }
    }

    impl<C, T> Collector<T> for Serial<C>
    where
        C: Collector<(usize, T)>,
    {
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            // Put this here because if the index is `usize::MAX`
            // and it's the last item the underlying can afford,
            // we should still be able to collect it and exit early
            // instead of panicking (in debug build).
            self.collector.collect((self.idx, item))?;
            self.idx += 1;
            ControlFlow::Continue(())
        }

        #[inline]
        unsafe fn assume_reserved_collect(&mut self, item: T) -> ControlFlow<()> {
            unsafe {
                // SAFETY: The caller reserved for at least 1 item.
                self.collector.assume_reserved_collect((self.idx, item))?;
            }

            self.idx += 1;
            ControlFlow::Continue(())
        }

        // We can't meaningfully override the other two methods,
        // because we need to uphold the "the index is `usize::MAX` and the last item"
        // case, which would lead us to a manual `try_fold()`,
        // which is the default implementation.
    }
}

#[cfg(test)]
mod proptests {
    use crate::test_utils::prelude::*;

    par_collector_test!(indexed {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let mut n_and_start = prop_oneof![..=2_usize, usize::MAX - 2..]
                .prop_flat_map(|start| (..=(usize::MAX - start).min(5), Just(start)));
        },
        iter: nums.par_iter().cloned(),
        collector: {
            let (n, start) = n_and_start;
            let mut collector = vec![].into_par_collector().take(n).enumerate();
            collector.idx = start;
            collector
        },
        starting_ma_f: |request| n_and_start.0.min(request),
        expected_f: |iter, count, request| {
            let (n, start) = n_and_start;
            let mut idx = start;

            let res: Vec<_> = iter
                .zip(std::iter::repeat_with(|| {
                    let old_idx = idx;
                    idx += 1;
                    old_idx
                }))
                .map(|(num, i)| (i, num))
                .take(n)
                .collect();

            (
                res,
                if let remaining @ 1.. = n.saturating_sub(count) {
                    Continue((idx, remaining.min(request)))
                } else {
                    Break(())
                },
            )
        },
        output_pred: PartialEq::eq,
        state_pred: |collector, &idx| collector.idx == idx,
    });
}
