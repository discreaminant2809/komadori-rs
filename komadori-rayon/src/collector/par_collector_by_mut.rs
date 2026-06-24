use super::IntoParallelCollectorBase;

/// A type that can be converted into a parallel collector by mutable reference.
///
/// This trait's main purpose is to provide a convenience method to creates
/// a parallel collector from `&mut T`.
///
/// You cannot implement this trait directly.
/// Instead, you should implement [`IntoParallelCollectorBase`] for `&mut T`
/// (where `T` is your type)
/// and this trait is automatically implemented for `T`.
///
/// This trait is not intended for use in bounds.
/// Use [`IntoParallelCollectorBase`] and similar traits in trait bounds instead.
///
/// # Examples
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::prelude::*;
///
/// let mut nums = vec![1, 2];
///
/// (3..=6)
///     .into_par_iter()
///     .feed_into(nums.par_collector_mut());
///
/// assert_eq!(nums, [1, 2, 3, 4, 5, 6]);
/// ```
///
/// If possible, you can use `&mut` instead of `.par_collector_mut()`:
///
/// ```
/// use rayon::prelude::*;
/// use komadori_rayon::prelude::*;
///
/// let mut nums = vec![1, 2];
///
/// (3..=6)
///     .into_par_iter()
///     .feed_into(&mut nums);
///
/// assert_eq!(nums, [1, 2, 3, 4, 5, 6]);
/// ```
pub trait ParallelCollectorByMut
where
    for<'a> &'a mut Self: IntoParallelCollectorBase,
{
    /// Creates a parallel collector from a shared reference of a value.
    #[inline]
    fn par_collector_mut(&mut self) -> <&'_ mut Self as IntoParallelCollectorBase>::IntoParCollector {
        self.into_par_collector()
    }
}
impl<T> ParallelCollectorByMut for T
where
    T: ?Sized,
    for<'a> &'a mut T: IntoParallelCollectorBase,
{
}
