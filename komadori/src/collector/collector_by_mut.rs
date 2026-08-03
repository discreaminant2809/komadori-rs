use super::IntoCollectorBase;

/// A type that can be converted into a collector by mutable reference.
///
/// This trait's main purpose is to provide a convenience method to creates
/// a collector from `&mut T`.
///
/// You cannot implement this trait directly.
/// Instead, you should implement [`IntoCollectorBase`] for `&mut T`
/// (where `T` is your type)
/// and this trait is automatically implemented for `T`.
///
/// This trait is not intended for use in bounds.
/// Use [`IntoCollector`] or [`IntoCollectorBase`] in trait bounds instead.
///
/// # Examples
///
/// ```
/// use komadori::prelude::*;
///
/// let mut nums1 = vec![];
/// let mut nums2 = vec![1, 2];
///
/// [3, 4, 5].into_iter().feed_into((
///     // Use reference if you don't chain adapters (shorter and more readable).
///     &mut nums1,
///     // Use `.collector_mut()` if you do chain adapters.
///     nums2.collector_mut().take(2),
/// ));
///
/// assert_eq!(nums1, [3, 4, 5]);
/// assert_eq!(nums2, [1, 2, 3, 4]);
/// ```
///
/// [`IntoCollector`]: super::IntoCollector
#[allow(private_bounds)]
pub trait CollectorByMut
where
    for<'a> &'a mut Self: IntoCollectorBase,
{
    /// Creates a collector from a mutable reference of a value.
    #[inline]
    fn collector_mut(&mut self) -> <&mut Self as IntoCollectorBase>::IntoCollector {
        self.into_collector()
    }
}

impl<T> CollectorByMut for T
where
    T: ?Sized,
    for<'a> &'a mut T: IntoCollectorBase,
{
}
