use crate::tuple::Tuple;

use super::{CallMut, CallOnce};

pub trait DefineCallOnce<'a, Binder: self_binder::Sealed = self_binder::Binder<'a, Self>> {
    type CallOnce;
}

pub trait DefineCallMut<'a, Binder: self_binder::Sealed = self_binder::Binder<'a, Self>> {
    type CallMut;
}

pub trait ParallelFnOnceBase: for<'a> DefineCallOnce<'a> {
    fn callable_once<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallOnce<'a>>::CallOnce + Clone + Send;

    #[inline]
    fn take_callable_once<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallOnce<'a>>::CallOnce + Clone + Send {
        self.callable_once()
    }
}

#[allow(dead_code)]
pub trait ParallelFnOnce<Args: Tuple>:
    ParallelFnOnceBase<CallOnce: CallOnce<Args, Output = Self::Output>>
{
    type Output;
}
impl<F, Args, R> ParallelFnOnce<Args> for F
where
    Args: Tuple,
    F: ParallelFnOnceBase<CallOnce: CallOnce<Args, Output = R>> + ?Sized,
{
    type Output = R;
}

pub trait ParallelFnMutBase: ParallelFnOnceBase + for<'a> DefineCallMut<'a> {
    fn callable_mut<'a>(&'a mut self)
    -> impl FnOnce() -> <Self as DefineCallMut<'a>>::CallMut + Clone + Send;

    #[inline]
    fn take_callable_mut<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallMut<'a>>::CallMut + Clone + Send {
        self.callable_mut()
    }
}

#[allow(dead_code)]
pub trait ParallelFnMut<Args: Tuple>:
    ParallelFnOnce<Args> + ParallelFnMutBase<CallMut: CallMut<Args, Output = Self::Output>>
{
}
impl<F, Args> ParallelFnMut<Args> for F
where
    Args: Tuple,
    F: ParallelFnOnce<Args> + ParallelFnMutBase<CallMut: CallMut<Args, Output = Self::Output>> + ?Sized,
{
}

/// Used for the hack. Should not be able to be referred outside.
mod self_binder {
    use std::marker::PhantomData;

    pub trait Sealed {}
    #[allow(missing_debug_implementations)]
    pub struct Binder<'a, T: ?Sized>(PhantomData<&'a mut T>);
    impl<'a, T: ?Sized> Sealed for Binder<'a, T> {}
}
