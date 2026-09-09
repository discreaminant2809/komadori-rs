use komadori::prelude::*;

pub trait DefineInner<'a, Binder = &'a mut Self> {
    type Inner: CollectorBase;
}

pub trait SplittableInner: for<'a> DefineInner<'a> {
    // Some do have a way to hint early. Two of them are `nest_serial()` and `try_fold_local()`.
    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        request
    }

    fn anchor<'a>(&'a mut self) -> impl Anchor<Inner = <Self as DefineInner<'a>>::Inner>;

    #[inline]
    fn take_anchor<'a>(&'a mut self) -> impl Anchor<Inner = <Self as DefineInner<'a>>::Inner> {
        self.anchor()
    }
}

pub trait Anchor: Clone + Send {
    type Inner: CollectorBase;

    fn into_inner(self) -> Self::Inner;

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        request
    }
}

impl<F, R> Anchor for F
where
    F: FnOnce() -> R + Clone + Send,
    R: IntoCollectorBase,
{
    type Inner = R::IntoCollector;

    #[inline]
    fn into_inner(self) -> Self::Inner {
        self().into_collector()
    }
}
