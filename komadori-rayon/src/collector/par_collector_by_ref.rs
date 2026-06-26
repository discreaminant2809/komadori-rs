use super::IntoParallelCollectorBase;

/// A type that can be converted into a parallel collector by shared reference.
///
/// This trait's main purpose is to provide a convenience method to creates
/// a parallel collector from `&T`.
///
/// You cannot implement this trait directly.
/// Instead, you should implement [`IntoParallelCollectorBase`] for `&T`
/// (where `T` is your type)
/// and this trait is automatically implemented for `T`.
///
/// This trait is not intended for use in bounds.
/// Use [`IntoParallelCollectorBase`] and similar traits in trait bounds instead.
pub trait ParallelCollectorByRef
where
    for<'a> &'a Self: IntoParallelCollectorBase,
{
    /// Creates a parallel collector from a shared reference of a value.
    #[inline]
    fn par_collector(&self) -> <&'_ Self as IntoParallelCollectorBase>::IntoParCollector {
        self.into_par_collector()
    }
}
impl<T> ParallelCollectorByRef for T
where
    T: ?Sized,
    for<'a> &'a T: IntoParallelCollectorBase,
{
}
