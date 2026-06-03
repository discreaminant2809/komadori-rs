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
