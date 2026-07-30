use std::ops::ControlFlow;

use crate::collector::{
    Collector, CollectorBase, Fuse, advanced_collect_many_default_impl, and_break,
    finish_boxed_impl,
};

#[derive(Debug, Clone)]
pub struct TeeBase<C1, C2, TF> {
    collector1: Fuse<C1>,
    collector2: Fuse<C2>,
    teer: TF,
}

impl<C1, C2, TF> TeeBase<C1, C2, TF>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    pub(super) fn new(collector1: C1, collector2: C2, teer: TF) -> Self {
        Self {
            collector1: collector1.fuse(),
            collector2: collector2.fuse(),
            teer,
        }
    }
}

pub(super) trait DefinePassDown<
    'this,
    T: ?Sized,
    Binder: t_binder::Sealed = t_binder::Binder<'this, T>,
>
{
    type PassDown;
}

/// Used for the hack. Should not be able to be referred outside.
mod t_binder {
    use std::marker::PhantomData;

    pub trait Sealed {}
    #[allow(missing_debug_implementations)]
    pub struct Binder<'a, T: ?Sized>(PhantomData<&'a mut T>);
    impl<'a, T: ?Sized> Sealed for Binder<'a, T> {}
}

pub(super) trait Teer<T>: for<'a> DefinePassDown<'a, T> {
    const TEE_CHEAP: bool = false;

    fn pass_down<'a>(&mut self, item: &'a mut T) -> <Self as DefinePassDown<'a, T>>::PassDown;

    #[inline]
    fn no_tee_collect(
        &mut self,
        collector: &mut impl for<'a> Collector<<Self as DefinePassDown<'a, T>>::PassDown>,
        item: T,
    ) -> ControlFlow<()> {
        let mut item = item;
        collector.collect(self.pass_down(&mut item))
    }

    #[inline]
    unsafe fn no_tee_assume_reserved_collect(
        &mut self,
        collector: &mut impl for<'a> Collector<<Self as DefinePassDown<'a, T>>::PassDown>,
        item: T,
    ) -> ControlFlow<()> {
        let mut item = item;
        unsafe { collector.assume_reserved_collect(self.pass_down(&mut item)) }
    }
}

impl<C1, C2, TF> CollectorBase for TeeBase<C1, C2, TF>
where
    C1: CollectorBase,
    C2: CollectorBase,
{
    type Output = (C1::Output, C2::Output);

    #[inline]
    fn finish(self) -> Self::Output {
        (self.collector1.finish(), self.collector2.finish())
    }

    finish_boxed_impl!();

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.collector1.reserve(additional);
        self.collector2.reserve(additional);
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        // `max`, not `min`.
        // Even if one stops, the other still proceeds.
        self.collector1
            .max_afford(request)
            .max(self.collector2.max_afford(request))
    }
}

impl<C1, C2, TF, T> Collector<T> for TeeBase<C1, C2, TF>
where
    TF: Teer<T>,
    C1: for<'a> Collector<<TF as DefinePassDown<'a, T>>::PassDown>,
    C2: Collector<T>,
{
    #[inline]
    fn collect(&mut self, mut item: T) -> ControlFlow<()> {
        if TF::TEE_CHEAP {
            let cf1 = self.collector1.collect(self.teer.pass_down(&mut item));
            let cf2 = self.collector2.collect(item);
            and_break(cf1, cf2)
        } else if self.collector2.max_afford(1) == 0 {
            self.teer.no_tee_collect(&mut self.collector1, item)
        } else if self.collector1.max_afford(1) == 0 {
            self.collector2.collect(item)
        } else {
            let cf1 = self.collector1.collect(self.teer.pass_down(&mut item));
            let cf2 = self.collector2.collect(item);
            and_break(cf1, cf2)
        }
    }

    #[inline]
    unsafe fn assume_reserved_collect(&mut self, mut item: T) -> ControlFlow<()> {
        unsafe {
            if TF::TEE_CHEAP {
                let cf1 = self
                    .collector1
                    .assume_reserved_collect(self.teer.pass_down(&mut item));
                let cf2 = self.collector2.assume_reserved_collect(item);
                and_break(cf1, cf2)
            } else if self.collector2.max_afford(1) == 0 {
                self.teer
                    .no_tee_assume_reserved_collect(&mut self.collector1, item)
            } else if self.collector1.max_afford(1) == 0 {
                self.collector2.assume_reserved_collect(item)
            } else {
                let cf1 = self
                    .collector1
                    .assume_reserved_collect(self.teer.pass_down(&mut item));
                let cf2 = self.collector2.assume_reserved_collect(item);
                and_break(cf1, cf2)
            }
        }
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        advanced_collect_many_default_impl(self, items)
    }

    // No meaningful override for this method.
    // fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {}
}
