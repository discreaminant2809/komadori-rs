use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that computes the minimum value among the items it collects.
///
/// Its [`Output`](ParallelCollectorBase::Output) is `None` if it has not collected any items,
/// or `Some` containing the minimum item otherwise.
///
/// This collector corresponds to [`Iterator::min()`].
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParMin};
///
/// let min = [4, 5, 2, 4, 3]
///     .into_par_iter()
///     .feed_into(ParMin::new());
///
/// assert_eq!(min, Some(2));
/// ```
///
/// The output is `None` if no items were collected.
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, cmp::ParMin};
///
/// let min = ([] as [i32; _])
///     .into_par_iter()
///     .feed_into(ParMin::new());
///
/// assert_eq!(min, None);
/// ```
#[derive(Debug, Clone)]
pub struct ParMin<T> {
    min: Option<T>,
}

impl<T> ParMin<T>
where
    T: Ord + Send,
{
    /// Creates a new instance of this parallel collector.
    #[inline]
    pub const fn new() -> Self {
        assert_unindexed_par_collector::<_, T>(Self { min: None })
    }
}

impl<T> Default for ParMin<T>
where
    T: Ord + Send,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'this, T> DefineSerial<'this> for ParMin<T>
where
    T: Ord + Send,
{
    type Serial = unique::Serial<'this, Self, consumer::Serial<T>>;
}

impl<'this, T> DefineUnindexedSerial<'this> for ParMin<T>
where
    T: Ord + Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, consumer::Serial<T>>;
}

impl<T> ParallelCollectorBase for ParMin<T>
where
    T: Ord + Send,
{
    type Output = Option<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.min
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
            combine(&mut self.min, output);
            ControlFlow::Continue(())
        }))
    }
}

impl<T> UnindexedParallelCollectorBase for ParMin<T>
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
            combine(&mut self.min, output);
            ControlFlow::Continue(())
        }))
    }
}

#[inline]
fn combine<T: Ord>(left: &mut Option<T>, right: Option<T>) {
    crate::iter::combine_opt(left, right, |left, right| {
        if right < *left {
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

    pub type Serial<T> = komadori::cmp::Min<T>;

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
