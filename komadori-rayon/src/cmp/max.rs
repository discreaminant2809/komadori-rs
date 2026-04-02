use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::collector::{
    ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector_base,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
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
        assert_unindexed_par_collector_base(Self { max: None })
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

impl<'this, T> DefineConsumer<'this> for ParMax<T>
where
    T: Ord + Send,
{
    type Consumer = consumer::Consumer<T>;
}

impl<'this, T> DefineUnindexedConsumer<'this> for ParMax<T>
where
    T: Ord + Send,
{
    type UnindexedConsumer = consumer::Consumer<T>;
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
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        let (consumer, commit) = self.parts_unindexed();
        (len, consumer, commit)
    }
}

impl<T> UnindexedParallelCollectorBase for ParMax<T>
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
            combine(&mut self.max, output);
            ControlFlow::Continue(())
        })
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

        type IntoCollector = komadori::cmp::Max<T>;

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
