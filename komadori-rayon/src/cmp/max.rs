use std::ops::ControlFlow;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, SerialOf, SerialOutputOf,
            UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that computes the maximum value among the items it collects.
///
/// Its [`Output`](ParallelCollectorBase::Output) is `None` if it has not collected any items,
/// or `Some` containing the maximum item otherwise.
///
/// This collector corresponds to [`Iterator::max()`].
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParMax};
///
/// let max = [1, 3, 2, 5, 3]
///     .into_par_iter()
///     .feed_into(ParMax::new());
///
/// assert_eq!(max, Some(5));
/// ```
///
/// The output is `None` if no items were collected.
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParMax};
///
/// let max = ([] as [i32; _])
///     .into_par_iter()
///     .feed_into(ParMax::new());
///
/// assert_eq!(max, None);
/// ```
#[derive(Debug, Clone)]
pub struct ParMax<T> {
    max: Option<T>,
}

impl<T> ParMax<T>
where
    T: Ord + Send,
{
    /// Creates a new instance of this parallel collector.
    #[inline]
    pub const fn new() -> Self {
        assert_unindexed_par_collector::<_, T>(Self { max: None })
    }
}

impl<T> Default for ParMax<T>
where
    T: Ord + Send,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'this, T> DefineSerial<'this> for ParMax<T>
where
    T: Ord + Send,
{
    type Serial = unique::Serial<'this, Self, consumer::Serial<T>>;
}

impl<'this, T> DefineUnindexedSerial<'this> for ParMax<T>
where
    T: Ord + Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, consumer::Serial<T>>;
}

impl<T> ParallelCollectorBase for ParMax<T>
where
    T: Ord + Send,
{
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.max
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((consumer::unindexed(), |output| {
            combine(&mut self.max, output);
            ControlFlow::Continue(())
        }))
    }
}

impl<T> UnindexedParallelCollectorBase for ParMax<T>
where
    T: Ord + Send,
{
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify((consumer::unindexed(), |output| {
            combine(&mut self.max, output);
            ControlFlow::Continue(())
        }))
    }
}

#[inline]
fn combine<T: Ord>(left: &mut Option<T>, right: Option<T>) {
    crate::iter::combine_opt(left, right, |left, right| {
        if right < *left {
        } else {
            *left = right;
        }
    });
}

mod consumer {
    use crate::collector::plumbing::{BasicUnindexedConsumer, OpaqueUnindexedConsumer};

    pub fn unindexed<T>() -> OpaqueUnindexedConsumer!(Serial<T>)
    where
        T: Ord + Send,
    {
        BasicUnindexedConsumer {
            state: (),
            split_f: |_| {},
            combiner_f: |_| super::combine,
            ma_f: |_, request| request,
            collector_f: |_| Serial::new(),
        }
    }

    pub type Serial<T> = komadori::cmp::Max<T>;
}
