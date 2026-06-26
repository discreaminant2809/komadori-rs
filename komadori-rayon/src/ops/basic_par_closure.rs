use std::fmt::Debug;

use super::{DefineCallMut, DefineCallOnce, ParallelFnMutBase, ParallelFnOnceBase};

/// A parallel closure that is just a wrapper of an [`Fn`].
#[derive(Clone)]
pub struct BasicParClosure<F> {
    f: F,
}

impl<F> BasicParClosure<F> {
    /// Creates a new instance of this parallel closure.
    #[inline]
    pub const fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F> Debug for BasicParClosure<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicParClosure")
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}

impl<'a, F> DefineCallOnce<'a> for BasicParClosure<F> {
    type CallOnce = call::Callable<'a, F>;
}

impl<'a, F> DefineCallMut<'a> for BasicParClosure<F> {
    type CallMut = call::Callable<'a, F>;
}

impl<F> ParallelFnOnceBase for BasicParClosure<F>
where
    F: Sync,
{
    fn callable_once<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallOnce<'a>>::CallOnce + Clone + Send {
        || call::Callable::new(&self.f)
    }
}

impl<F> ParallelFnMutBase for BasicParClosure<F>
where
    F: Sync,
{
    fn callable_mut<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallMut<'a>>::CallMut + Clone + Send {
        || call::Callable::new(&self.f)
    }
}

#[allow(missing_debug_implementations)]
mod call {
    use crate::{
        ops::{Call, CallMut, CallOnce},
        tuple::Tuple,
    };

    pub struct Callable<'a, F>(&'a F);

    impl<'a, F> Callable<'a, F> {
        #[inline]
        pub(super) fn new(f: &'a F) -> Self {
            Self(f)
        }
    }

    impl<F, Args> CallOnce<Args> for Callable<'_, F>
    where
        Args: Tuple,
        F: Call<Args>,
    {
        type Output = F::Output;

        #[inline]
        fn call_once(self, args: Args) -> Self::Output {
            self.0.call(args)
        }
    }

    impl<F, Args> CallMut<Args> for Callable<'_, F>
    where
        Args: Tuple,
        F: Call<Args>,
    {
        #[inline]
        fn call_mut(&mut self, args: Args) -> Self::Output {
            self.0.call(args)
        }
    }
}

fn _assert<R>(f: impl Fn() -> R + Sync) {
    fn _assert_par_fn_mut<R>(_: impl super::ParallelFnMut<(), Output = R>) {}
    _assert_par_fn_mut::<R>(BasicParClosure::new(f));
}
