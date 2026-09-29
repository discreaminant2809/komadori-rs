#![forbid(unsafe_code, reason = "Prevent misuses of `MaybeUninit`")]

use std::{fmt::Debug, mem::MaybeUninit, ops::ControlFlow, sync::atomic::AtomicBool};

use crate::{
    collector::{
        ParallelCollectorBase, UnindexedParallelCollectorBase,
        plumbing::{
            Consumer, DefineSerial, DefineUnindexedSerial, SerialOf, SerialOutputOf,
            UnindexedConsumer, UnindexedSerialOf, UnindexedSerialOutputOf,
        },
    },
    ops::ChangeOutputType,
};

use super::Try;

pub struct ParTrying<C: Try> {
    collector: Option<C::Output>,
    residual: Option<C::Residual>,
    stopped: MaybeUninit<AtomicBool>,
}

impl<C: Try> ParTrying<C> {
    pub fn new(collector: C) -> Self {
        match collector.branch() {
            ControlFlow::Continue(collector) => Self {
                collector: Some(collector),
                residual: None,
                stopped: MaybeUninit::uninit(),
            },
            ControlFlow::Break(residual) => Self {
                collector: None,
                residual: Some(residual),
                stopped: MaybeUninit::uninit(),
            },
        }
    }
}

impl<C> Clone for ParTrying<C>
where
    C: Try<Output: Clone, Residual: Clone>,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            collector: self.collector.clone(),
            residual: self.residual.clone(),
            stopped: MaybeUninit::uninit(),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.collector.clone_from(&source.collector);
        self.residual.clone_from(&source.residual);
    }
}

impl<C> Debug for ParTrying<C>
where
    C: Try<Output: Debug, Residual: Debug>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParTrying")
            .field("collector", &self.collector)
            .field("residual", &self.residual)
            .field("stopped", &self.stopped)
            .finish()
    }
}

impl<'a, C> DefineSerial<'a> for ParTrying<C>
where
    C: Try<Output: DefineUnindexedSerial<'a>, Residual: Send>,
{
    // It's just an internal type so we don't need to wrap it in uniquify helper.
    type Serial = <Self as DefineUnindexedSerial<'a>>::UnindexedSerial;
}

impl<'a, C> DefineUnindexedSerial<'a> for ParTrying<C>
where
    C: Try<Output: DefineUnindexedSerial<'a>, Residual: Send>,
{
    // It's just an internal type so we don't need to wrap it in uniquify helper.
    type UnindexedSerial = consumer::Serial<
        'a,
        ChangeOutputType<C, <C::Output as DefineUnindexedSerial<'a>>::UnindexedSerial>,
    >;
}

impl<C> ParallelCollectorBase for ParTrying<C>
where
    C: Try<Output: UnindexedParallelCollectorBase, Residual: Send>,
{
    type Output = ChangeOutputType<C, <C::Output as ParallelCollectorBase>::Output>;

    #[inline]
    fn finish(self) -> Self::Output {
        if let Some(residual) = self.residual {
            Try::from_residual(residual)
        } else if let Some(collector) = self.collector {
            Try::from_output(collector.finish())
        } else {
            unreachable!("`collector` and `residual` cannot both be None")
        }
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        if self.residual.is_some() {
            0
        } else if let Some(collector) = &self.collector {
            collector.max_afford(request)
        } else {
            unreachable!("`collector` and `residual` cannot both be None")
        }
    }

    fn parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>) -> ControlFlow<()>,
    ) {
        self.unindexed_parts()
    }

    fn take_parts<'a>(
        &'a mut self,
        _len: usize,
    ) -> (
        impl Consumer<IntoCollector = SerialOf<'a, Self>, Output = SerialOutputOf<'a, Self>>,
        impl FnOnce(SerialOutputOf<'a, Self>),
    ) {
        self.take_unindexed_parts()
    }
}

impl<C> UnindexedParallelCollectorBase for ParTrying<C>
where
    C: Try<Output: UnindexedParallelCollectorBase, Residual: Send>,
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
        if self.residual.is_some() {
            (
                consumer::Consumer::<ChangeOutputType<C, _>>::Break,
                committer(&mut self.residual, None),
            )
        } else if let Some(collector) = &mut self.collector {
            let (consumer, commit) = collector.unindexed_parts();
            let stopped = self.stopped.write(AtomicBool::new(false));

            (
                consumer::Consumer::Continue { consumer, stopped },
                committer(&mut self.residual, Some(commit)),
            )
        } else {
            unreachable!("`collector` and `residual` cannot both be None")
        }
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
        if self.residual.is_some() {
            (
                consumer::Consumer::<ChangeOutputType<C, _>>::Break,
                take_committer(&mut self.residual, None),
            )
        } else if let Some(collector) = &mut self.collector {
            let (consumer, commit) = collector.take_unindexed_parts();
            let stopped = self.stopped.write(AtomicBool::new(false));

            (
                consumer::Consumer::Continue { consumer, stopped },
                take_committer(&mut self.residual, Some(commit)),
            )
        } else {
            unreachable!("`collector` and `residual` cannot both be None")
        }
    }
}

// The closure must have the same type, so we factor this out.
#[inline]
fn committer<R, O>(
    residual: &mut Option<R>,
    commit: Option<impl FnOnce(O) -> ControlFlow<()>>,
) -> impl FnOnce(ControlFlow<Option<R>, O>) -> ControlFlow<()> {
    |output| match (output, commit) {
        (ControlFlow::Continue(output), Some(commit)) => commit(output),
        (ControlFlow::Break(r), _) => {
            *residual = r;
            ControlFlow::Break(())
        }
        _ => unreachable!("if a serial output presents, there must be a committer"),
    }
}

#[inline]
fn take_committer<R, O>(
    residual: &mut Option<R>,
    commit: Option<impl FnOnce(O)>,
) -> impl FnOnce(ControlFlow<Option<R>, O>) {
    |output| match (output, commit) {
        (ControlFlow::Continue(output), Some(commit)) => commit(output),
        (ControlFlow::Break(r), _) => *residual = r,
        _ => unreachable!("if a serial output presents, there must be a committer"),
    }
}

#[expect(missing_debug_implementations)]
mod consumer {
    use std::{
        marker::PhantomData,
        ops::ControlFlow,
        sync::atomic::{AtomicBool, Ordering},
    };

    use komadori::collector::CollectorBase;

    use crate::{
        collector::plumbing::{self, IntoCollectorBase},
        ops::{ChangeOutputType, Try},
    };

    pub enum Consumer<'a, C: Try> {
        Continue {
            consumer: C::Output,
            stopped: &'a AtomicBool,
        },
        Break,
    }

    pub enum Serial<'a, C: Try> {
        Continue {
            collector: C::Output,
            stopped: &'a AtomicBool,
        },
        Break(Option<C::Residual>),
    }

    type Output<C> =
        ControlFlow<Option<<C as Try>::Residual>, <<C as Try>::Output as CollectorBase>::Output>;

    impl<'a, C> IntoCollectorBase for Consumer<'a, C>
    where
        C: Try<Output: IntoCollectorBase>,
    {
        type Output = Output<ChangeOutputType<C, <C::Output as IntoCollectorBase>::IntoCollector>>;

        type IntoCollector =
            Serial<'a, ChangeOutputType<C, <C::Output as IntoCollectorBase>::IntoCollector>>;

        #[inline]
        fn into_collector(self) -> Self::IntoCollector {
            match self {
                Self::Continue { consumer, stopped } => Serial::Continue {
                    collector: consumer.into_collector(),
                    stopped,
                },
                Self::Break => Serial::Break(None),
            }
        }
    }

    impl<'a, C> plumbing::Consumer for Consumer<'a, C>
    where
        C: Try<Output: plumbing::UnindexedConsumer, Residual: Send>,
    {
        plumbing::impl_split_at_via_unindexed!('a, C);

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            match self {
                Self::Continue { consumer, stopped } if !stopped.load(Ordering::Relaxed) => {
                    consumer.max_afford(request)
                }
                _ => 0,
            }
        }
    }

    impl<'a, C> plumbing::UnindexedConsumer for Consumer<'a, C>
    where
        C: Try<Output: plumbing::UnindexedConsumer, Residual: Send>,
    {
        #[inline]
        fn split_off_left(&self) -> Self {
            match self {
                Self::Continue { consumer, stopped } => Self::Continue {
                    consumer: consumer.split_off_left(),
                    stopped,
                },
                Self::Break => Self::Break,
            }
        }

        #[inline]
        fn to_combiner(&self) -> impl FnOnce(&mut Self::Output, Self::Output) + use<'a, C> {
            let combiner_state = match self {
                Self::Continue { consumer, stopped } => Some((consumer.to_combiner(), *stopped)),
                Self::Break => None,
            };

            move |left, right| {
                let Some((combine, stopped)) = combiner_state else {
                    // If this is `None` then nothing was processed at all!
                    return;
                };

                match (left, right) {
                    (ControlFlow::Continue(left), ControlFlow::Continue(right))
                        if !stopped.load(Ordering::Relaxed) =>
                    {
                        combine(left, right)
                    }
                    (left @ ControlFlow::Continue(_), right @ ControlFlow::Break(_)) => {
                        *left = right
                    }
                    (ControlFlow::Break(left @ None), ControlFlow::Break(right)) => *left = right,
                    _ => {}
                }
            }
        }
    }

    impl<'a, C> CollectorBase for Serial<'a, C>
    where
        C: Try<Output: CollectorBase>,
    {
        type Output = Output<C>;

        #[inline]
        fn finish(self) -> Self::Output {
            match self {
                Self::Continue { stopped, .. } if stopped.load(Ordering::Relaxed) => {
                    ControlFlow::Break(None)
                }
                Self::Continue { collector, .. } => ControlFlow::Continue(collector.finish()),
                Self::Break(residual) => ControlFlow::Break(residual),
            }
        }

        plumbing::finish_boxed_impl! {}

        #[inline]
        fn max_afford(&self, request: usize) -> usize {
            match self {
                Self::Continue { collector, stopped } if !stopped.load(Ordering::Relaxed) => {
                    collector.max_afford(request)
                }
                _ => 0,
            }
        }
    }

    // `PhantomData` is mandatory here or else the compiler will complain `T` being uncontrained.
    // True, tho. The implementor of `Residual` may not use `T` at all.
    impl<'a, C, T> plumbing::Collector<(ChangeOutputType<C, T>, PhantomData<T>)> for Serial<'a, C>
    where
        C: Try<Output: plumbing::Collector<T>>,
    {
        #[inline]
        fn collect(
            &mut self,
            (item, _): (ChangeOutputType<C, T>, PhantomData<T>),
        ) -> ControlFlow<()> {
            match (self, item.branch()) {
                (this @ Self::Continue { .. }, ControlFlow::Break(residual)) => {
                    *this = Self::Break(Some(residual));
                    ControlFlow::Break(())
                }
                (Self::Continue { stopped, .. }, _) if stopped.load(Ordering::Relaxed) => {
                    ControlFlow::Break(())
                }
                (Self::Continue { collector, .. }, ControlFlow::Continue(item)) => {
                    collector.collect(item)
                }
                (Self::Break(_), _) => ControlFlow::Break(()),
            }
        }

        #[inline]
        fn collect_many(
            &mut self,
            items: impl IntoIterator<Item = (ChangeOutputType<C, T>, PhantomData<T>)>,
        ) -> ControlFlow<()> {
            match self {
                Self::Continue { stopped, .. } if stopped.load(Ordering::Relaxed) => {
                    ControlFlow::Break(())
                }
                Self::Continue { collector, stopped } => {
                    match items.into_iter().try_for_each(|(item, _)| {
                        collector
                            .collect(item.branch().map_break(Some)?)
                            .map_break(|_| None)?;

                        if stopped.load(Ordering::Relaxed) {
                            ControlFlow::Break(None)
                        } else {
                            ControlFlow::Continue(())
                        }
                    }) {
                        ControlFlow::Continue(()) => ControlFlow::Continue(()),
                        ControlFlow::Break(residual) => {
                            if let Some(residual) = residual {
                                *self = Self::Break(Some(residual));
                            }

                            ControlFlow::Break(())
                        }
                    }
                }
                Self::Break(_) => ControlFlow::Break(()),
            }
        }

        #[inline]
        fn collect_then_finish(
            self,
            items: impl IntoIterator<Item = (ChangeOutputType<C, T>, PhantomData<T>)>,
        ) -> Self::Output {
            enum BreakReason<R> {
                CollectorStopped,
                OtherLeaf,
                FoundResidual(R),
            }

            match self {
                Self::Continue { stopped, .. } if stopped.load(Ordering::Relaxed) => {
                    ControlFlow::Break(None)
                }
                Self::Continue {
                    mut collector,
                    stopped,
                } => {
                    match items.into_iter().try_for_each(|(item, _)| {
                        collector
                            .collect(item.branch().map_break(BreakReason::FoundResidual)?)
                            .map_break(|_| BreakReason::CollectorStopped)?;

                        if stopped.load(Ordering::Relaxed) {
                            ControlFlow::Break(BreakReason::OtherLeaf)
                        } else {
                            ControlFlow::Continue(())
                        }
                    }) {
                        ControlFlow::Continue(())
                        | ControlFlow::Break(BreakReason::CollectorStopped) => {
                            ControlFlow::Continue(collector.finish())
                        }
                        ControlFlow::Break(BreakReason::OtherLeaf) => ControlFlow::Break(None),
                        ControlFlow::Break(BreakReason::FoundResidual(residual)) => {
                            ControlFlow::Break(Some(residual))
                        }
                    }
                }
                Self::Break(residual) => ControlFlow::Break(residual),
            }
        }
    }
}
