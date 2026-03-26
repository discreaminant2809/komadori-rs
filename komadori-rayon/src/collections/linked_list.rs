//! Parallel collectors for [`LinkedList`].
//!
//! This module corresponds to [`std::collections::linked_list`].

use std::{collections::LinkedList, ops::ControlFlow};

use komadori::prelude::*;

use crate::collector::{
    IntoParallelCollectorBase, ParallelCollectorBase, UnindexedParallelCollectorBase,
    assert_unindexed_par_collector,
    plumbing::{DefineConsumer, DefineUnindexedConsumer},
};

/// A parallel collector that pushes collected items into a [`LinkedList`].
/// Its [`Output`] is [`LinkedList`].
///
/// This struct is created by `LinkedList::into_par_collector()`.
///
/// [`Output`]: ParallelCollectorBase::Output
#[derive(Debug, Clone)]
pub struct IntoParCollector<T>(LinkedList<T>);

/// A parallel collector that pushes collected items into a
/// [`&mut LinkedList`](LinkedList).
/// Its [`Output`] is [`&mut LinkedList`](LinkedList).
///
/// This struct is created by `LinkedList::par_collector_mut()`.
///
/// [`Output`]: ParallelCollectorBase::Output
#[derive(Debug)]
pub struct ParCollectorMut<'a, T>(&'a mut LinkedList<T>);

impl<T> Default for IntoParCollector<T>
where
    T: Send,
{
    #[inline]
    fn default() -> Self {
        LinkedList::default().into_par_collector()
    }
}

impl<T> IntoParallelCollectorBase for LinkedList<T>
where
    T: Send,
{
    type Output = Self;

    type IntoParCollector = IntoParCollector<T>;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        assert_unindexed_par_collector::<_, T>(IntoParCollector(self))
    }
}

impl<'a, T> IntoParallelCollectorBase for &'a mut LinkedList<T>
where
    T: Send,
{
    type Output = Self;

    type IntoParCollector = ParCollectorMut<'a, T>;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        assert_unindexed_par_collector::<_, T>(ParCollectorMut(self))
    }
}

impl<'this, T> DefineConsumer<'this> for IntoParCollector<T>
where
    T: Send,
{
    type Consumer = __internal::Consumer<T>;
}

impl<T> ParallelCollectorBase for IntoParCollector<T>
where
    T: Send,
{
    type Output = LinkedList<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }

    #[inline]
    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> std::ops::ControlFlow<()>,
    ) {
        let (consumer, commit) = self.parts_unindexed();
        (len, consumer, commit)
    }
}

impl<'this, T> DefineUnindexedConsumer<'this> for IntoParCollector<T>
where
    T: Send,
{
    type UnindexedConsumer = __internal::Consumer<T>;
}

impl<T> UnindexedParallelCollectorBase for IntoParCollector<T>
where
    T: Send,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        (__internal::Consumer::new(), |mut output| {
            self.0.append(&mut output);
            ControlFlow::Continue(())
        })
    }
}

impl<'this, 'c, T> DefineConsumer<'this> for ParCollectorMut<'c, T>
where
    T: Send,
{
    type Consumer = __internal::Consumer<T>;
}

impl<'c, T> ParallelCollectorBase for ParCollectorMut<'c, T>
where
    T: Send,
{
    type Output = &'c mut LinkedList<T>;

    #[inline]
    fn finish(self) -> Self::Output {
        self.0
    }

    #[inline]
    fn parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        <Self as DefineConsumer<'a>>::Consumer,
        impl FnOnce(
            <<Self as DefineConsumer<'a>>::Consumer as IntoCollectorBase>::Output,
        ) -> std::ops::ControlFlow<()>,
    ) {
        let (consumer, commit) = self.parts_unindexed();
        (len, consumer, commit)
    }
}

impl<'this, 'c, T> DefineUnindexedConsumer<'this> for ParCollectorMut<'c, T>
where
    T: Send,
{
    type UnindexedConsumer = __internal::Consumer<T>;
}

impl<'c, T> UnindexedParallelCollectorBase for ParCollectorMut<'c, T>
where
    T: Send,
{
    fn parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        <Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer,
        impl FnOnce(
            <<Self as DefineUnindexedConsumer<'a>>::UnindexedConsumer as IntoCollectorBase>::Output,
        ) -> ControlFlow<()>,
    ) {
        (__internal::Consumer::new(), |mut output| {
            self.0.append(&mut output);
            ControlFlow::Continue(())
        })
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub mod __internal {
    use std::{collections::LinkedList, ops::ControlFlow};

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumerBase};

    pub struct Consumer<T>(LinkedList<T>);

    pub struct Combiner(());

    impl<T> Consumer<T> {
        #[inline]
        pub(super) fn new() -> Self {
            Self(LinkedList::new())
        }
    }

    impl<T> CollectorBase for Consumer<T> {
        type Output = LinkedList<T>;

        #[inline]
        fn finish(self) -> Self::Output {
            self.0
        }
    }

    impl<T> Collector<T> for Consumer<T> {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            self.0.push_back(item);
            ControlFlow::Continue(())
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            self.0.extend(items);
            ControlFlow::Continue(())
        }
    }

    impl<T> plumbing::ConsumerBase for Consumer<T>
    where
        T: Send,
    {
        type Combiner = Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }
    }

    impl<T> plumbing::UnindexedConsumerBase for Consumer<T>
    where
        T: Send,
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

    impl<T> plumbing::Combiner<LinkedList<T>> for Combiner {
        #[inline]
        fn combine(self, left: &mut LinkedList<T>, mut right: LinkedList<T>) {
            left.append(&mut right);
        }
    }
}
