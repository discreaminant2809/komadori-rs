use std::{fmt::Debug, ops::ControlFlow};

use komadori::prelude::*;

use crate::collector::{
    ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
};

/// A parallel collector that reduces all collected items into a single value
/// by repeatedly applying a reduction function.
///
/// If no items have been collected, its [`Output`](ParallelCollectorBase::Output) is `None`;
/// otherwise, it returns `Some` containing the result of the reduction.
///
/// This collector corresponds to [`Iterator::reduce()`], except the closure is
/// the "left" value mutated by the "right" value instead of the two values
/// producing another value. Also, the application order is unspecified rather
/// than strictly from left to right, but it is still guaranteed that when
/// two items are fed into the closure, the first one is left compared to
/// the second one (the "right" value).
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParReduce};
///
/// let res = [3, 2, 5, 1, 4]
///     .into_par_iter()
///     .feed_into(ParReduce::new(|accum, num| *accum += num));
///
/// assert_eq!(res, Some(15));
/// ```
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParReduce};
///
/// let res = ([] as [i32; _])
///     .into_par_iter()
///     .feed_into(ParReduce::new(|accum, num| *accum += num));
///
/// assert_eq!(res, None);
/// ```
#[derive(Clone)]
pub struct ParReduce<T, F> {
    accum: Option<T>,
    f: F,
}

impl<T: Debug, F> Debug for ParReduce<T, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Reduce")
            .field("accum", &self.accum)
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

impl<T, F> ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    /// Creates a new instance of this parallel collector with a given accumulator.
    #[inline]
    pub const fn new(f: F) -> Self {
        assert_unindexed_par_collector::<_, T>(Self { accum: None, f })
    }
}

impl<'this, T, F> DefineConsumer<'this> for ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    type Consumer = consumer::Consumer<T, &'this F>;
}

impl<T, F> ParallelCollectorBase for ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.accum
    }

    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.parts_unindexed();
        (len, consumer, commit)
    }
}

impl<'this, T, F> DefineUnindexedConsumer<'this> for ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    type UnindexedConsumer = consumer::Consumer<T, &'this F>;
}

impl<T, F> UnindexedParallelCollectorBase for ParReduce<T, F>
where
    T: Send,
    F: Fn(&mut T, T) + Sync,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ) -> std::ops::ControlFlow<()>,
    ) {
        (consumer::Consumer::new(&self.f), |output| {
            combine(&self.f, &mut self.accum, output);
            ControlFlow::Continue(())
        })
    }
}

fn combine<T>(f: impl FnOnce(&mut T, T), left: &mut Option<T>, right: Option<T>) {
    match (left, right) {
        (_, None) => {}
        (left @ None, Some(right)) => *left = Some(right),
        (Some(left), Some(right)) => f(left, right),
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::marker::PhantomData;

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumerBase};

    pub struct Consumer<T, F> {
        f: F,
        _marker: PhantomData<T>,
    }

    pub struct Combiner<F> {
        f: F,
    }

    impl<T, F> Consumer<T, F> {
        pub(super) fn new(f: F) -> Self {
            Self {
                f,
                _marker: PhantomData,
            }
        }
    }

    impl<T, F> IntoCollectorBase for Consumer<T, F>
    where
        F: FnMut(&mut T, T),
    {
        type Output = Option<T>;

        type IntoCollector = komadori::iter::Reduce<T, F>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Self::IntoCollector::new(self.f)
        }
    }

    impl<T, F> plumbing::ConsumerBase for Consumer<T, F>
    where
        T: Send,
        F: FnMut(&mut T, T) + Clone + Send,
    {
        type Combiner = Combiner<F>;

        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }
    }

    impl<T, F> plumbing::UnindexedConsumerBase for Consumer<T, F>
    where
        T: Send,
        F: FnMut(&mut T, T) + Clone + Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                f: self.f.clone(),
                _marker: PhantomData,
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            Combiner { f: self.f.clone() }
        }
    }

    impl<F, T> plumbing::Combiner<Option<T>> for Combiner<F>
    where
        F: FnMut(&mut T, T),
    {
        #[inline]
        fn combine(self, left: &mut Option<T>, right: Option<T>) {
            super::combine(self.f, left, right);
        }
    }
}
