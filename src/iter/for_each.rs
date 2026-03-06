use std::{fmt::Debug, ops::ControlFlow};

use crate::collector::{Collector, CollectorBase};

/// A collector that calls a provided closure for each collected item.
///
/// This collector corresponds to [`Iterator::for_each()`].
///
/// # Examples
///
/// ```
/// use komadori::{prelude::*, iter::ForEach};
/// use std::sync::mpsc;
///
/// let (tx, rx) = mpsc::channel();
///
/// (0..5).map(|x| x * 2 + 1)
///       .feed_into(ForEach::new(move |x| tx.send(x).unwrap()));
///
/// let v: Vec<_> = rx.iter().collect();
/// assert_eq!(v, [1, 3, 5, 7, 9]);
/// ```
#[derive(Clone)]
pub struct ForEach<F> {
    f: F,
}

impl<F> ForEach<F> {
    /// Creates a new instance of this collector with a closure.
    #[inline]
    pub fn new<T>(f: F) -> Self
    where
        F: FnMut(T),
    {
        Self { f }
    }
}

impl<F> CollectorBase for ForEach<F> {
    type Output = ();

    #[inline]
    fn finish(self) -> Self::Output {}
}

impl<F, T> Collector<T> for ForEach<F>
where
    F: FnMut(T),
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        (self.f)(item);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        items.into_iter().for_each(&mut self.f);
        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_then_finish(self, items: impl IntoIterator<Item = T>) -> Self::Output {
        items.into_iter().for_each(self.f);
    }
}

impl<F> Debug for ForEach<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForEach")
            .field("f", &std::any::type_name::<F>())
            .finish()
    }
}
