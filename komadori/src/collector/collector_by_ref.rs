use super::IntoCollectorBase;

/// A type that can be converted into a collector by shared reference.
///
/// This trait's main purpose is to provide a convenience method to creates
/// a collector from `&T`.
///
/// You cannot implement this trait directly.
/// Instead, you should implement [`IntoCollectorBase`] for `&T`
/// (where `T` is your type)
/// and this trait is automatically implemented for `T`.
///
/// This trait is not intended for use in bounds.
/// Use [`IntoCollector`] or [`IntoCollectorBase`] in trait bounds instead.
///
/// # Examples
///
/// ```
/// use std::sync::mpsc;
/// use komadori::prelude::*;
///
/// let (tx, rx) = mpsc::channel();
///
/// [1; 3].into_iter().feed_into((
///     // Use `.collector()` if you do chain adapters.
///     tx.collector().copying(),
///     // Use reference if you don't chain adapters (shorter and more readable).
///     &tx,
/// ));
///
/// let nums = rx.try_iter().feed_into(vec![]);
/// assert_eq!(nums, [1; 6]);
/// ```
///
/// [`IntoCollector`]: super::IntoCollector
#[allow(private_bounds)]
pub trait CollectorByRef
where
    for<'a> &'a Self: IntoCollectorBase,
{
    /// Creates a collector from a shared reference of a value.
    #[inline]
    fn collector(&self) -> <&Self as IntoCollectorBase>::IntoCollector {
        self.into_collector()
    }
}

impl<T> CollectorByRef for T
where
    T: ?Sized,
    for<'a> &'a T: IntoCollectorBase,
{
}
