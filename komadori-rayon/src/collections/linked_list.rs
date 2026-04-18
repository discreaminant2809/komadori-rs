//! Parallel collectors for [`LinkedList`].
//!
//! This module corresponds to [`std::collections::linked_list`].

use std::{collections::LinkedList, ops::ControlFlow};

use komadori::prelude::*;

use crate::{
    collector::{
        IntoParallelCollectorBase, ParallelCollectorBase, UnindexedParallelCollectorBase,
        assert_unindexed_par_collector,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
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

impl<'this, T> DefineSerial<'this> for IntoParCollector<T>
where
    T: Send,
{
    type Serial = unique::Serial<'this, Self, consumer::Serial<T>>;
}

impl<'this, T> DefineUnindexedSerial<'this> for IntoParCollector<T>
where
    T: Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, consumer::Serial<T>>;
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
        unique::uniquify((len, consumer::Consumer::new(), |mut output| {
            self.0.append(&mut output);
            ControlFlow::Continue(())
        }))
    }
}

impl<T> UnindexedParallelCollectorBase for IntoParCollector<T>
where
    T: Send,
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
        unique_unindexed::uniquify((consumer::Consumer::new(), |mut output| {
            self.0.append(&mut output);
            ControlFlow::Continue(())
        }))
    }
}

impl<'this, 'c, T> DefineSerial<'this> for ParCollectorMut<'c, T>
where
    T: Send,
{
    type Serial = unique::Serial<'this, Self, consumer::Serial<T>>;
}

impl<'this, 'c, T> DefineUnindexedSerial<'this> for ParCollectorMut<'c, T>
where
    T: Send,
{
    type UnindexedSerial = unique_unindexed::Serial<'this, Self, consumer::Serial<T>>;
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
        unique::uniquify((len, consumer::Consumer::new(), |mut output| {
            self.0.append(&mut output);
            ControlFlow::Continue(())
        }))
    }
}

impl<'c, T> UnindexedParallelCollectorBase for ParCollectorMut<'c, T>
where
    T: Send,
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
        unique_unindexed::uniquify((consumer::Consumer::new(), |mut output| {
            self.0.append(&mut output);
            ControlFlow::Continue(())
        }))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::collections::LinkedList;

    use komadori::prelude::*;

    use crate::collector::plumbing::{self, UnindexedConsumer};

    pub struct Consumer<T>(LinkedList<T>);

    pub struct Combiner(());

    pub type Serial<T> = <LinkedList<T> as IntoCollectorBase>::IntoCollector;

    impl<T> Consumer<T> {
        #[inline]
        pub(super) fn new() -> Self {
            Self(LinkedList::new())
        }
    }

    impl<T> IntoCollectorBase for Consumer<T> {
        type Output = LinkedList<T>;

        type IntoCollector = Serial<T>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            self.0.into_collector()
        }
    }

    impl<T> plumbing::Consumer for Consumer<T>
    where
        T: Send,
    {
        type Combiner = Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }
    }

    impl<T> plumbing::UnindexedConsumer for Consumer<T>
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
