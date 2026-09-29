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
    ops::{AdvancedParClosure, BasicParClosure, DefineCallMut, ParallelFnMutBase},
};

mod private {
    #[derive(Debug, Clone)]
    pub struct ParForEachBase<F> {
        pub(super) f: F,
    }
}
use private::ParForEachBase;

/// A parallel collector that calls a provided function for each collected item.
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

/// Same as [`ParForEach`], but with a shared state and a
/// clonable "consumer." Each leaf reduction receives a shared reference
/// to the shared state and a function created by the consumer
/// specifically for this leaf.
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::{prelude::*, iter::ParForEachWith};
/// use komadori::prelude::*;
/// use std::sync::mpsc::channel;
///
/// let (sender, receiver) = channel();
///
/// [1, 22, 20, 4444]
///     .into_par_iter()
///     .feed_into(ParForEachWith::new((), move |_| {
///         let mut buf = String::new();
///         move |_, num| {
///             // I know, this is not an efficient way to
///             // count the number of digits.
///             // This is just an example.
///             buf.clear();
///             use std::fmt::Write;
///             write!(buf, "{num}");
///
///             sender.send(buf.len()).unwrap();
///         }
///     }));
///
/// let mut nums = receiver.iter().feed_into(vec![]);
/// nums.sort_unstable();
///
/// assert_eq!(nums, [1, 2, 2, 4]);
/// ```
pub type ParForEachWith<S, FF> = ParForEachBase<AdvancedParClosure<S, FF>>;

impl<F> ParForEach<F> {
    /// Creates a new instance of this collector with a function.
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
}

impl<S, FF> ParForEachWith<S, FF> {
    /// Creates a new instance of this collector with a "consumer"
    /// and a shared state.
    ///
    /// This parallel collector collects `T`.
    #[inline]
    pub fn new<F, T>(shared_state: S, consumer: FF) -> Self
    where
        S: Sync,
        FF: FnOnce(&S) -> F + Clone + Send,
        F: FnMut(&S, T),
    {
        assert_unindexed_par_collector::<_, T>(Self {
            f: AdvancedParClosure::new(shared_state, consumer),
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
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        unique::uniquify((consumer::unindexed(self.f.callable_mut()), |_| {
            ControlFlow::Continue(())
        }))
    }

    fn take_parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        unique::take_uniquify((consumer::unindexed(self.f.take_callable_mut()), |_| {}))
    }
}

impl<F> UnindexedParallelCollectorBase for ParForEachBase<F>
where
    F: ParallelFnMutBase,
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
        unique_unindexed::uniquify((consumer::unindexed(self.f.callable_mut()), |_| {
            ControlFlow::Continue(())
        }))
    }

    fn take_unindexed_parts<'a>(
        &'a mut self,
    ) -> (
        impl UnindexedConsumer<
            IntoCollector = UnindexedSerialOf<'a, Self>,
            Output = UnindexedSerialOutputOf<'a, Self>,
        >,
        impl FnOnce(UnindexedSerialOutputOf<'a, Self>),
    ) {
        unique_unindexed::take_uniquify((consumer::unindexed(self.f.take_callable_mut()), |_| {}))
    }
}

#[expect(missing_debug_implementations)]
mod consumer {
    use std::ops::ControlFlow;

    use komadori::prelude::*;

    use crate::{
        collector::plumbing::{BasicUnindexedConsumer, OpaqueUnindexedConsumer, finish_boxed_impl},
        ops::CallMut,
    };

    pub fn unindexed<F>(
        into_f: impl FnOnce() -> F + Clone + Send,
    ) -> OpaqueUnindexedConsumer!(Serial<F>) {
        BasicUnindexedConsumer {
            state: into_f,
            split_f: Clone::clone,
            combiner_f: |_| |_, _| {},
            ma_f: |_, request| request,
            collector_f: |into_f| Serial { f: into_f() },
        }
    }

    pub struct Serial<F> {
        f: F,
    }

    impl<F> CollectorBase for Serial<F> {
        type Output = ();

        #[inline]
        fn finish(self) -> Self::Output {}

        finish_boxed_impl! {}
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
            items
                .into_iter()
                .for_each(move |item| self.f.call_mut((item,)));
        }
    }
}
