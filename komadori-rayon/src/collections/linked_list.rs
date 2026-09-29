//! Parallel collectors for [`LinkedList`].
//!
//! This module corresponds to [`std::collections::linked_list`].

use std::{collections::LinkedList, ops::ControlFlow};

use komadori::prelude::*;

use crate::{
    collector::{
        IntoParallelCollectorBase, ParallelCollectorBase, UnindexedParallelCollectorBase,
        assert_unindexed_par_collector,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, SerialOf, SerialOutputOf,
            UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    helpers::{unique, unique_unindexed},
};

/// A parallel collector that pushes collected items into a [`LinkedList`].
/// Its [`Output`] is [`LinkedList`].
///
/// This can collect `T` where `T` is [`Send`],
/// and `&T` and `&mut T` where `T` is [`Send`] and [`Copy`].
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
/// This can collect `T` where `T` is [`Send`],
/// and `&T` and `&mut T` where `T` is [`Send`] and [`Copy`].
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
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((consumer::unindexed(), |mut output| {
            self.0.append(&mut output);
            ControlFlow::Continue(())
        }))
    }
}

impl<T> UnindexedParallelCollectorBase for IntoParCollector<T>
where
    T: Send,
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
        unique_unindexed::uniquify((consumer::unindexed(), |mut output| {
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
        _len: usize,
    ) -> (
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output) -> ControlFlow<()>,
    ) {
        unique::uniquify((consumer::unindexed(), |mut output| {
            self.0.append(&mut output);
            ControlFlow::Continue(())
        }))
    }
}

impl<'c, T> UnindexedParallelCollectorBase for ParCollectorMut<'c, T>
where
    T: Send,
{
    fn unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(
            <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        ) -> ControlFlow<()>,
    ){
        unique_unindexed::uniquify((consumer::unindexed(), |mut output| {
            self.0.append(&mut output);
            ControlFlow::Continue(())
        }))
    }
}

mod consumer {
    use std::collections::LinkedList;

    use crate::collector::plumbing::{
        BasicUnindexedConsumer, IntoCollectorBase, OpaqueUnindexedConsumer,
    };

    pub fn unindexed<T>() -> OpaqueUnindexedConsumer!(Serial<T>)
    where
        T: Send,
    {
        BasicUnindexedConsumer {
            state: (),
            split_f: |_| {},
            combiner_f: |_| |left: &mut LinkedList<T>, mut right| left.append(&mut right),
            ma_f: |_, request| request,
            collector_f: |_| LinkedList::new().into_collector(),
        }
    }

    pub type Serial<T> = <LinkedList<T> as IntoCollectorBase>::IntoCollector;
}

#[cfg(test)]
mod proptests {
    use crate::test_utils::prelude::*;

    par_collector_test!(indexed {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let mut starting_nums = proptest::collection::linked_list(any::<i32>(), ..=2);
        },
        iter: nums.par_iter().cloned(),
        collector: starting_nums.into_par_collector(),
        starting_ma_f: |request| request,
        expected_f: |iter, _, request| (
            {
                let mut ret = starting_nums.clone();
                ret.extend(iter);
                ret
            },
            Continue(((), request))
        ),
        output_pred: PartialEq::eq,
        state_pred: state_is_irrelevant(),
    });

    unindexed_par_collector_test!(unindexed {
        iter_data: {
            let mut nums = propvec(any::<i32>(), ..=5);
        },
        other_data: {
            let mut starting_nums = proptest::collection::linked_list(any::<i32>(), ..=2);
        },
        iter: nums.par_iter().cloned(),
        collector: starting_nums.into_par_collector(),
        starting_ma_f: |request| request,
        expected_f: |iter, _, request| (
            {
                let mut ret = starting_nums.clone();
                ret.extend(iter);
                ret
            },
            Continue(((), request))
        ),
        output_pred: PartialEq::eq,
        state_pred: state_is_irrelevant(),
    });
}
