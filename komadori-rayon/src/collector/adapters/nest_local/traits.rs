use std::ops::ControlFlow;

use komadori::prelude::*;

pub trait DefineLocal<'a, Binder = &'a mut Self> {
    type Local: CollectorBase;
}

pub trait SplittableLocal: for<'a> DefineLocal<'a> {
    // Some do have a way to hint early. Two of them are `nest_serial()` and `try_fold_local()`.
    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn anchor<'a>(&'a mut self) -> impl Anchor<Inner = <Self as DefineLocal<'a>>::Local>;

    #[inline]
    fn take_anchor<'a>(&'a mut self) -> impl Anchor<Inner = <Self as DefineLocal<'a>>::Local> {
        self.anchor()
    }
}

pub trait Anchor: Clone + Send {
    type Inner;

    fn into_inner(self) -> Self::Inner;

    #[inline]
    fn break_hint(&self) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
}
