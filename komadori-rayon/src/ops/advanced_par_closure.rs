use std::{any::type_name, fmt::Debug};

use super::{DefineCallMut, DefineCallOnce, ParallelFnMutBase, ParallelFnOnceBase};

/// A paralle closure that creates a (local) state from a function.
///
/// See [`par_closure!`](crate::par_closure) for more.
#[derive(Clone)]
pub struct AdvancedParClosure<S, FF> {
    shared_state: S,
    consumer: Option<FF>,
}

impl<S, FF> AdvancedParClosure<S, FF> {
    #[inline]
    pub const fn new(shared_state: S, consumer: FF) -> Self {
        Self {
            shared_state,
            consumer: Some(consumer),
        }
    }
}

impl<S, FF> Debug for AdvancedParClosure<S, FF>
where
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdvancedParClosure")
            .field("shared_state", &self.shared_state)
            .field("consumer", &type_name::<FF>())
            .finish()
    }
}

impl<'a, S, FF, F> DefineCallOnce<'a> for AdvancedParClosure<S, FF>
where
    S: Sync,
    FF: FnOnce(&S) -> F + Clone + Send,
{
    type CallOnce = call::Callable<'a, S, F>;
}

impl<'a, S, FF, F> DefineCallMut<'a> for AdvancedParClosure<S, FF>
where
    S: Sync,
    FF: FnOnce(&S) -> F + Clone + Send,
{
    type CallMut = call::Callable<'a, S, F>;
}

impl<S, FF, F> ParallelFnOnceBase for AdvancedParClosure<S, FF>
where
    S: Sync,
    FF: FnOnce(&S) -> F + Clone + Send,
{
    fn callable_once<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallOnce<'a>>::CallOnce + Clone + Send {
        let consumer = self.consumer.clone().expect(TAKEN_ERR_MSG);
        || call::Callable::new(&self.shared_state, consumer(&self.shared_state))
    }

    fn take_callable_once<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallOnce<'a>>::CallOnce + Clone + Send {
        let consumer = self.consumer.take().expect(TAKEN_ERR_MSG);
        || call::Callable::new(&self.shared_state, consumer(&self.shared_state))
    }
}

impl<S, FF, F> ParallelFnMutBase for AdvancedParClosure<S, FF>
where
    S: Sync,
    FF: FnOnce(&S) -> F + Clone + Send,
{
    fn callable_mut<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallMut<'a>>::CallMut + Clone + Send {
        let consumer = self.consumer.clone().expect(TAKEN_ERR_MSG);
        || call::Callable::new(&self.shared_state, consumer(&self.shared_state))
    }

    fn take_callable_mut<'a>(
        &'a mut self,
    ) -> impl FnOnce() -> <Self as DefineCallMut<'a>>::CallMut + Clone + Send {
        let consumer = self.consumer.take().expect(TAKEN_ERR_MSG);
        || call::Callable::new(&self.shared_state, consumer(&self.shared_state))
    }
}

const TAKEN_ERR_MSG: &str = "`consumer` is already taken";

#[expect(missing_debug_implementations)]
mod call {
    use crate::{
        ops::{CallMut, CallOnce},
        tuple::PushFrontTuple,
    };

    pub struct Callable<'a, S, F> {
        shared_state: &'a S,
        f: F,
    }

    impl<'a, S, F> Callable<'a, S, F> {
        #[inline]
        pub(super) fn new(shared_state: &'a S, f: F) -> Self {
            Self { shared_state, f }
        }
    }

    impl<S, F, Args, R> CallOnce<Args> for Callable<'_, S, F>
    where
        Args: PushFrontTuple,
        F: for<'a> CallOnce<Args::PushFront<&'a S>, Output = R>,
    {
        type Output = R;

        #[inline]
        fn call_once(self, args: Args) -> Self::Output {
            self.f.call_once(args.push_front(self.shared_state))
        }
    }

    impl<S, F, Args, R> CallMut<Args> for Callable<'_, S, F>
    where
        Args: PushFrontTuple,
        F: for<'a> CallMut<Args::PushFront<&'a S>, Output = R>,
    {
        #[inline]
        fn call_mut(&mut self, args: Args) -> Self::Output {
            self.f.call_mut(args.push_front(self.shared_state))
        }
    }
}
