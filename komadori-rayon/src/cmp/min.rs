use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::{
    ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
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

impl<'this, T> DefineConsumer<'this> for ParMin<T>
where
    T: Ord + Send,
{
    type Consumer = consumer::Consumer<T>;
}

impl<'this, T> DefineUnindexedConsumer<'this> for ParMin<T>
where
    T: Ord + Send,
{
    type UnindexedConsumer = consumer::Consumer<T>;
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
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.parts_unindexed();
        (len, consumer, commit)
    }
}

impl<T> UnindexedParallelCollectorBase for ParMin<T>
where
    T: Ord + Send,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        (consumer::Consumer::new(), |output| {
            combine(&mut self.min, output);
            ControlFlow::Continue(())
        })
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

    use crate::collector::plumbing::{self, UnindexedConsumerBase};

    pub struct Consumer<T>(PhantomData<T>);

    pub struct Combiner(());

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

        type IntoCollector = komadori::cmp::Min<T>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Self::IntoCollector::new()
        }
    }

    impl<T> plumbing::ConsumerBase for Consumer<T>
    where
        T: Ord + Send,
    {
        type Combiner = Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }
    }

    impl<T> UnindexedConsumerBase for Consumer<T>
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
