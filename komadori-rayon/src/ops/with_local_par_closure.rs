use std::{any::type_name, fmt::Debug};

use super::{Call, DefineCallMut, DefineCallOnce, ParallelFnMutBase, ParallelFnOnceBase};

/// A paralle closure that creates a (local) state from a function.
///
/// See [`par_closure!`](crate::par_closure) for more.
#[derive(Clone)]
pub struct WithLocalParClosure<L1, FL2, F> {
    local1: Option<L1>,
    local2_f: FL2,
    f: F,
}

impl<L1, FL2, F> WithLocalParClosure<L1, FL2, F> {
    #[inline]
    pub const fn new(local1: L1, local2_f: FL2, f: F) -> Self {
        Self {
            local1: Some(local1),
            local2_f,
            f,
        }
    }
}

impl<L1, FL2, F> Debug for WithLocalParClosure<L1, FL2, F>
where
    L1: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParClosureWithStateFn")
            .field("local1", &self.local1)
            .field("state_f", &type_name::<FL2>())
            .field("f", &type_name::<F>())
            .finish()
    }
}

impl<'a, L1, FL2, F> DefineCallOnce<'a> for WithLocalParClosure<L1, FL2, F>
where
    L1: Clone + Send,
    FL2: Call<()> + Sync,
    F: Sync,
{
    type CallOnce = call_once::Callable<'a, L1, FL2::Output, F>;
}

impl<'a, L1, FL2, F> DefineCallMut<'a> for WithLocalParClosure<L1, FL2, F>
where
    L1: Clone + Send,
    FL2: Call<()> + Sync,
    F: Sync,
{
    type CallMut = call_mut::Callable<'a, L1, FL2::Output, F>;
}

impl<L1, FL2, F> ParallelFnOnceBase for WithLocalParClosure<L1, FL2, F>
where
    L1: Clone + Send,
    FL2: Call<()> + Sync,
    F: Sync,
{
    fn callable_once<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallOnce<'a>>::CallOnce + Clone + Send {
        let local1 = self.local1.as_ref().expect("local1 has been taken").clone();
        || call_once::Callable::new(local1, self.local2_f.call(()), &self.f)
    }

    fn take_callable_once<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallOnce<'a>>::CallOnce + Clone + Send {
        let local1 = self.local1.take().expect("local1 has been taken");
        || call_once::Callable::new(local1, self.local2_f.call(()), &self.f)
    }
}

impl<L1, FL2, F> ParallelFnMutBase for WithLocalParClosure<L1, FL2, F>
where
    L1: Clone + Send,
    FL2: Call<()> + Sync,
    F: Sync,
{
    fn callable_mut<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallMut<'a>>::CallMut + Clone + Send {
        let local1 = self.local1.as_ref().expect("local1 has been taken").clone();
        || call_mut::Callable::new(local1, self.local2_f.call(()), &self.f)
    }

    fn take_callable_mut<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallMut<'a>>::CallMut + Clone + Send {
        let local1 = self.local1.take().expect("local1 has been taken");
        || call_mut::Callable::new(local1, self.local2_f.call(()), &self.f)
    }
}

#[allow(missing_debug_implementations)]
mod call_once {
    use crate::{
        ops::{Call, CallOnce},
        tuple::PushFront2Tuple,
    };

    pub struct Callable<'a, L1, L2, F> {
        local1: L1,
        local2: L2,
        f: &'a F,
    }

    impl<'a, L1, L2, F> Callable<'a, L1, L2, F> {
        pub(super) fn new(local1: L1, local2: L2, f: &'a F) -> Self {
            Self { local1, local2, f }
        }
    }

    impl<L1, L2, F, Args> CallOnce<Args> for Callable<'_, L1, L2, F>
    where
        Args: PushFront2Tuple,
        F: Call<Args::PushFront2<L1, L2>>,
    {
        type Output = F::Output;

        fn call_once(self, args: Args) -> Self::Output {
            self.f.call(args.push_front2(self.local1, self.local2))
        }
    }
}

#[allow(missing_debug_implementations)]
mod call_mut {
    use crate::{
        ops::{Call, CallMut, CallOnce},
        tuple::PushFront2Tuple,
    };

    pub struct Callable<'a, L1, L2, F> {
        local1: L1,
        local2: L2,
        f: &'a F,
    }

    impl<'a, L1, L2, F> Callable<'a, L1, L2, F> {
        pub(super) fn new(local1: L1, local2: L2, f: &'a F) -> Self {
            Self { local1, local2, f }
        }
    }

    impl<L1, L2, F, R, Args> CallOnce<Args> for Callable<'_, L1, L2, F>
    where
        Args: PushFront2Tuple,
        F: for<'l1, 'l2> Call<Args::PushFront2<&'l1 mut L1, &'l2 mut L2>, Output = R>,
    {
        type Output = R;

        fn call_once(mut self, args: Args) -> Self::Output {
            self.f.call(args.push_front2(&mut self.local1, &mut self.local2))
        }
    }

    impl<L1, L2, F, R, Args> CallMut<Args> for Callable<'_, L1, L2, F>
    where
        Args: PushFront2Tuple,
        F: for<'l1, 'l2> Call<Args::PushFront2<&'l1 mut L1, &'l2 mut L2>, Output = R>,
    {
        fn call_mut(&mut self, args: Args) -> Self::Output {
            self.f.call(args.push_front2(&mut self.local1, &mut self.local2))
        }
    }
}
