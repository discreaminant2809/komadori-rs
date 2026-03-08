use std::{
    fmt::{Debug, DebugStruct},
    ops::ControlFlow,
};

use crate::collector::{Collector, CollectorBase};

pub struct IsSortedBase<T, S> {
    state: State<T, S>,
}

pub(super) trait IsSortedStore<T, K> {
    fn map(&mut self, item: T) -> K;
    fn store(&mut self, prev: &mut K, item: T) -> bool;

    fn store_many(&mut self, prev: &mut K, mut items: impl Iterator<Item = T>) -> bool {
        items
            .try_for_each(move |item| {
                if self.store(prev, item) {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(())
                }
            })
            .is_continue()
    }
}

enum State<K, S> {
    StillSorted { prev: Option<K>, store: S },
    NotSorted,
}

impl<K, S> IsSortedBase<K, S> {
    pub fn new(store: S) -> Self {
        Self {
            state: State::StillSorted { prev: None, store },
        }
    }

    pub fn debug_state(&self, dbg_store: impl Fn(&mut DebugStruct, &S)) -> impl Debug
    where
        K: Debug,
    {
        struct DebugState<'a, T, P, F> {
            state: &'a State<T, P>,
            dbg_pred: F,
        }

        impl<T, P, F> Debug for DebugState<'_, T, P, F>
        where
            T: Debug,
            F: Fn(&mut DebugStruct, &P),
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.state {
                    State::StillSorted { prev, store } => {
                        let mut dbg_struct = f.debug_struct("StillSorted");
                        dbg_struct.field("prev", prev);
                        (self.dbg_pred)(&mut dbg_struct, store);
                        dbg_struct.finish()
                    }
                    State::NotSorted => f.write_str("NotSorted"),
                }
            }
        }

        DebugState {
            state: &self.state,
            dbg_pred: dbg_store,
        }
    }
}

impl<K, S> CollectorBase for IsSortedBase<K, S> {
    type Output = bool;

    #[inline]
    fn finish(self) -> Self::Output {
        matches!(self.state, State::StillSorted { .. })
    }
}

impl<T, K, S> Collector<T> for IsSortedBase<K, S>
where
    S: IsSortedStore<T, K>,
{
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match &mut self.state {
            State::StillSorted {
                prev: prev @ None,
                store,
            } => {
                *prev = Some(store.map(item));
                ControlFlow::Continue(())
            }

            State::StillSorted {
                prev: Some(prev),
                store,
            } => {
                if store.store(prev, item) {
                    ControlFlow::Continue(())
                } else {
                    self.state = State::NotSorted;
                    ControlFlow::Break(())
                }
            }

            State::NotSorted => ControlFlow::Break(()),
        }
    }

    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        let mut items = items.into_iter();

        match &mut self.state {
            State::StillSorted { prev, store } => {
                let prev = if let Some(prev) = prev {
                    prev
                } else if let Some(item) = items.next() {
                    prev.insert(store.map(item))
                } else {
                    return ControlFlow::Continue(());
                };

                if store.store_many(prev, items) {
                    ControlFlow::Continue(())
                } else {
                    self.state = State::NotSorted;
                    ControlFlow::Break(())
                }
            }

            State::NotSorted => ControlFlow::Break(()),
        }
    }

    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let mut items = items.into_iter();

        match self.state {
            State::StillSorted { prev, mut store } => {
                let mut prev = if let Some(prev) = prev {
                    prev
                } else if let Some(item) = items.next() {
                    store.map(item)
                } else {
                    return true;
                };

                store.store_many(&mut prev, items)
            }

            State::NotSorted => false,
        }
    }
}
