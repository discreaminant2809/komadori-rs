use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
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
        len: usize,
    ) -> (
        usize,
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        unique::uniquify((len, consumer::Consumer::new(), |output| {
            combine(&mut self.max, output);
            ControlFlow::Continue(())
        }))
    }
}

impl<T> UnindexedParallelCollectorBase for ParMax<T>
where
    T: Ord + Send,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        unique_unindexed::uniquify((consumer::Consumer::new(), |output| {
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

#[allow(missing_debug_implementations)]
mod consumer {
    use std::marker::PhantomData;

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumer};

    pub struct Consumer<T>(PhantomData<T>);

    pub struct Combiner(());

    pub type Serial<T> = komadori::cmp::Max<T>;

    impl<T> Consumer<T> {
        #[inline]
        pub(super) fn new() -> Self {
            Self(PhantomData)
        }
    }

    impl<T> IntoCollectorBase for Consumer<T>
    where
        T: Ord,
    {
        type Output = Option<T>;

        type IntoCollector = Serial<T>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Self::IntoCollector::new()
        }
    }

    impl<T> plumbing::Consumer for Consumer<T>
    where
        T: Ord + Send,
    {
        type Combiner = Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }
    }

    impl<T> UnindexedConsumer for Consumer<T>
    where
        T: Ord + Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self::new()
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            Combiner(())
        }
    }

    impl<T> plumbing::Combiner<Option<T>> for Combiner
    where
        T: Ord,
    {
        #[inline]
        fn combine(self, left: &mut Option<T>, right: Option<T>) {
            super::combine(left, right);
        }
    }
}
