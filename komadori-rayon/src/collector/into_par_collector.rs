use super::{
    ParallelCollector, ParallelCollectorBase, UnindexedParallelCollector, UnindexedParallelCollectorBase,
};

/// Conversion into a parallel collector.
///
/// By implementing this trait for a type, you define how it will be converted
/// to a parallel collector.
///
/// # Usage in trait bounds
///
/// Using `IntoParallelCollectorBase` in trait bounds allows a function to
/// be generic over both [`ParallelCollectorBase`] and `IntoParallelCollectorBase`.
/// This is convenient for users of the function, so when they are using it
/// they do not have to make an extra call to
/// [`into_par_collector()`](Self::into_par_collector) to obtain
/// an instance of a parallel collector.
///
/// Prefer [`IntoParallelCollector`], [`IntoUnindexedParallelCollector`]
/// and [`IntoUnindexedParallelCollectorBase`] whenever possible.
/// [`IntoParallelCollector`] and [`IntoUnindexedParallelCollector`]
/// can specify the item type more easily.
pub trait IntoParallelCollectorBase {
    /// The output of the parallel collector.
    type Output;

    /// Which parallel collector being produced?
    type IntoParCollector: ParallelCollectorBase<Output = Self::Output>;

    /// Creates a parallel collector from a value.
    fn into_par_collector(self) -> Self::IntoParCollector;
}

/// A trait alias for [`IntoParallelCollectorBase`] types that has
/// its parallel collector implement [`ParallelCollector`].
///
/// This trait is automatically implemented for such types.
///
/// Users generally should prefer this bound if they want to specify the
/// item type they need, instead of writing
/// `C: IntoParallelCollectorBase<IntoParCollector: ParallelCollector<T>>`.
pub trait IntoParallelCollector<T>:
    IntoParallelCollectorBase<IntoParCollector: ParallelCollector<T>>
{
}
impl<C, T> IntoParallelCollector<T> for C where
    C: IntoParallelCollectorBase<IntoParCollector: ParallelCollector<T>>
{
}

/// A trait alias for [`IntoParallelCollectorBase`] types that has
/// its parallel collector implement [`UnindexedParallelCollectorBase`].
///
/// This trait is automatically implemented for such types.
///
/// Users generally should prefer this bound if they want
/// an unindexed parallel collector, instead of writing
/// `C: IntoParallelCollectorBase<IntoParCollector: UnindexedParallelCollectorBase>`.
pub trait IntoUnindexedParallelCollectorBase:
    IntoParallelCollectorBase<IntoParCollector: UnindexedParallelCollectorBase>
{
}
impl<C> IntoUnindexedParallelCollectorBase for C where
    C: IntoParallelCollectorBase<IntoParCollector: UnindexedParallelCollectorBase>
{
}

/// A trait alias for [`IntoParallelCollectorBase`] types that has
/// its parallel collector implement [`UnindexedParallelCollector`].
///
/// This trait is automatically implemented for such types.
///
/// Users generally should prefer this bound if they want to specify the
/// item type they need, instead of writing
/// `C: IntoParallelCollectorBase<IntoParCollector: UnindexedParallelCollector<T>>`.
pub trait IntoUnindexedParallelCollector<T>:
    IntoUnindexedParallelCollectorBase<IntoParCollector: UnindexedParallelCollector<T>>
{
}
impl<C, T> IntoUnindexedParallelCollector<T> for C where
    C: IntoUnindexedParallelCollectorBase<IntoParCollector: UnindexedParallelCollector<T>>
{
}

impl<C> IntoParallelCollectorBase for C
where
    C: ParallelCollectorBase,
{
    type Output = C::Output;

    type IntoParCollector = C;

    #[inline]
    fn into_par_collector(self) -> Self::IntoParCollector {
        self
    }
}

fn _unindexed_substitutable_to_indexed<C, T>(x: C)
where
    C: IntoUnindexedParallelCollector<T>,
{
    fn check<C, T>(_: C)
    where
        C: IntoParallelCollector<T>,
    {
    }
    check::<C, T>(x);
}
