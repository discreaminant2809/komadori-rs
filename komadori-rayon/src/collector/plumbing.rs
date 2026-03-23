//!

use std::ops::ControlFlow;

use komadori::prelude::*;

///
pub trait DefineConsumer<'this, Binder: self_binder::Sealed = self_binder::Binder<'this, Self>>:
    Sized
{
    ///
    type Consumer: ConsumerBase;
}

///
pub trait DefineUnindexedConsumer<
    'this,
    Binder: self_binder::Sealed = self_binder::Binder<'this, Self>,
>: Sized
{
    ///
    type UnindexedConsumer: UnindexedConsumerBase;
}

mod self_binder {
    use std::marker::PhantomData;

    pub trait Sealed {}
    #[allow(missing_debug_implementations)]
    pub struct Binder<'a, T>(PhantomData<&'a mut T>);
    impl<'a, T> Sealed for Binder<'a, T> {}
}

///
pub trait ConsumerBase: IntoCollectorBase<Output: Send> + Send + Sized {
    ///
    type Combiner: Combiner<Self::Output>;

    ///
    fn split_off_left_at(&mut self, index: usize) -> (Self, Self::Combiner);

    ///
    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
}

///
pub trait UnindexedConsumerBase: ConsumerBase {
    ///
    fn split_off_left(&self) -> Self;

    ///
    fn to_combiner(&self) -> Self::Combiner;
}

///
pub trait Combiner<O> {
    ///
    fn combine(self, left: &mut O, right: O);
}

///
pub trait Consumer<T>: ConsumerBase<IntoCollector: Collector<T>> {}
impl<C, T> Consumer<T> for C where C: ConsumerBase<IntoCollector: Collector<T>> {}

///
pub trait UnindexedConsumer<T>: UnindexedConsumerBase<IntoCollector: Collector<T>> {}
impl<C, T> UnindexedConsumer<T> for C where C: UnindexedConsumerBase<IntoCollector: Collector<T>> {}

fn _unindexed_substitutable_to_indexed<T>(consumer: impl UnindexedConsumer<T>) {
    fn check_consumer<T>(_: impl Consumer<T>) {}
    check_consumer::<T>(consumer);
}
