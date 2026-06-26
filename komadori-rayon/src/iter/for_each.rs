use std::ops::ControlFlow;

use komadori::prelude::*;

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase, assert_unindexed_par_collector,
        plumbing::{Consumer, DefineSerial, DefineUnindexedSerial, UnindexedConsumer},
    },
    helpers::{unique, unique_unindexed},
    ops::{BasicParClosure, DefineCallMut, ParallelFnMutBase, WithLocalParClosure},
};

mod private {
    #[derive(Debug, Clone)]
    pub struct ParForEachBase<F> {
        pub(super) f: F,
    }
}
use private::ParForEachBase;

/// A parallel collector that calls a provided closure for each collected item.
///
/// This parallel collector corresponds to [`Iterator::for_each()`].
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParForEach};
/// use std::sync::Mutex;
///
/// // So that we won't get a bit nicer print instead of multiple mangled lines.
/// let lock = Mutex::new(());
///
/// (0..5)
///     .into_par_iter()
///     .feed_into(ParForEach::new(move |x| {
///         let _guard = lock.lock().unwrap();
///         println!("Got a number: {x}");
///     }));
/// ```
pub type ParForEach<F> = ParForEachBase<BasicParClosure<F>>;

/// Same as [`ParForEach`], but with a state that will either be cloned
/// or created from a factory (or both) to each serial execution.
///
// We can't link to `ParForEach::with` directly due to false
// `#[warn(rustdoc::private_intra_doc_links)]`.
/// This `struct` is created by [`ParForEach::with()`](ParForEach).
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParForEach};
/// use komadori::prelude::*;
/// use std::sync::mpsc::channel;
///
/// let (sender, receiver) = channel();
///
/// (1..=5)
///     .into_par_iter()
///     .feed_into(ParForEach::with(
///         sender, || {},
///         |sender, _, i| sender.send(i).unwrap(),
///     ));
///
/// let mut nums = receiver.iter().feed_into(vec![]);
/// nums.sort_unstable();
///
/// assert_eq!(nums, [1, 2, 3, 4, 5]);
/// ```
pub type ParForEachWith<L1, FL2, F> = ParForEachBase<WithLocalParClosure<L1, FL2, F>>;

impl<F> ParForEach<F> {
    /// Creates a new instance of this collector with a closure.
    ///
    /// This parallel collector collects `T`.
    #[inline]
    pub fn new<T>(f: F) -> Self
    where
        F: Fn(T) + Sync,
    {
        assert_unindexed_par_collector::<_, T>(Self {
            f: BasicParClosure::new(f),
        })
    }

    /// Creates a new instance of this collector with
    /// a state to be cloned, a factory of another state, and a closure.
    ///
    /// This parallel collector collects `T`.
    #[inline]
    pub fn with<L1, FL2, L2, T>(local1: L1, local2_f: FL2, f: F) -> ParForEachWith<L1, FL2, F>
    where
        L1: Clone + Send,
        FL2: Fn() -> L2 + Sync,
        F: Fn(&mut L1, &mut L2, T) + Sync,
    {
        assert_unindexed_par_collector::<_, T>(ParForEachBase {
            f: WithLocalParClosure::new(local1, local2_f, f),
        })
    }
}

impl<'a, F> DefineSerial<'a> for ParForEachBase<F>
where
    F: DefineCallMut<'a>,
{
    type Serial = unique::Serial<'a, Self, consumer::Serial<F::CallMut>>;
}

impl<'a, F> DefineUnindexedSerial<'a> for ParForEachBase<F>
where
    F: DefineCallMut<'a>,
{
    type UnindexedSerial = unique_unindexed::Serial<'a, Self, consumer::Serial<F::CallMut>>;
}

impl<F> ParallelCollectorBase for ParForEachBase<F>
where
    F: ParallelFnMutBase,
{
    type Output = ();

    #[inline]
    fn finish(self) -> Self::Output {}

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
        unique::uniquify((len, consumer::Consumer::new(self.f.callable_mut()), |_| {
            ControlFlow::Continue(())
        }))
    }

    fn take_parts<'a>(
        &'a mut self,
        len: usize,
    ) -> (
        usize,
        impl Consumer<
            IntoCollector = <Self as DefineSerial<'a>>::Serial,
            Output = <<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineSerial<'a>>::Serial as CollectorBase>::Output),
    ) {
        unique::take_uniquify((len, consumer::Consumer::new(self.f.take_callable_mut()), |_| {}))
    }
}

impl<F> UnindexedParallelCollectorBase for ParForEachBase<F>
where
    F: ParallelFnMutBase,
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
        unique_unindexed::uniquify((consumer::Consumer::new(self.f.callable_mut()), |_| {
            ControlFlow::Continue(())
        }))
    }

    fn take_parts_unindexed<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial,
            Output = <<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output,
        >,
        impl FnOnce(<<Self as DefineUnindexedSerial<'a>>::UnindexedSerial as CollectorBase>::Output),
    ) {
        unique_unindexed::take_uniquify((consumer::Consumer::new(self.f.take_callable_mut()), |_| {}))
    }
}

#[allow(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::{
        collector::plumbing::{self, UnindexedConsumer},
        ops::CallMut,
    };

    pub struct Consumer<FF> {
        into_f: FF,
    }

    pub struct Serial<F> {
        f: F,
    }

    pub struct Combiner;

    impl<FF> Consumer<FF> {
        #[inline]
        pub(super) fn new(into_f: FF) -> Self {
            Self { into_f }
        }
    }

    impl<FF, F> IntoCollectorBase for Consumer<FF>
    where
        FF: FnOnce() -> F,
    {
        type Output = ();

        type IntoCollector = Serial<F>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            Serial { f: (self.into_f)() }
        }
    }

    impl<FF, F> plumbing::Consumer for Consumer<FF>
    where
        FF: FnOnce() -> F + Clone + Send,
    {
        type Combiner = Combiner;

        #[inline]
        fn split_off_left_at(&mut self, _index: usize) -> (Self, Self::Combiner) {
            (self.split_off_left(), self.to_combiner())
        }
    }

    impl<FF, F> plumbing::UnindexedConsumer for Consumer<FF>
    where
        FF: FnOnce() -> F + Clone + Send,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            Self {
                into_f: self.into_f.clone(),
            }
        }

        #[inline]
        fn to_combiner(&self) -> Self::Combiner {
            Combiner
        }
    }

    impl plumbing::Combiner<()> for Combiner {
        #[inline]
        fn combine(self, _left: &mut (), _right: ()) {}
    }

    impl<F> CollectorBase for Serial<F> {
        type Output = ();

        #[inline]
        fn finish(self) -> Self::Output {}
    }

    impl<F, T> Collector<T> for Serial<F>
    where
        F: CallMut<(T,), Output = ()>,
    {
        #[inline]
        fn collect(&mut self, item: T) -> ControlFlow<()> {
            self.f.call_mut((item,));
            ControlFlow::Continue(())
        }

        #[inline]
        fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
            items.into_iter().for_each(|item| self.f.call_mut((item,)));
            ControlFlow::Continue(())
        }

        #[inline]
        fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
            items.into_iter().for_each(move |item| self.f.call_mut((item,)));
        }
    }
}
