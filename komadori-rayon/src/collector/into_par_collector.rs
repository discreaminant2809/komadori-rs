use super::{IndexedParallelCollector, ParallelCollector, ParallelCollectorBase};

///
pub trait IntoParallelCollectorBase {
    ///
    type Output;

    ///
    type IntoParCollector: ParallelCollectorBase<Output = Self::Output>;

    ///
    fn into_par_collector(self) -> Self::IntoParCollector;
}

///
pub trait IntoIndexedParallelCollector<T>:
    IntoParallelCollectorBase<IntoParCollector: IndexedParallelCollector<T>>
{
}
impl<C, T> IntoIndexedParallelCollector<T> for C where
    C: IntoParallelCollectorBase<IntoParCollector: IndexedParallelCollector<T>>
{
}

///
pub trait IntoParallelCollector<T>:
    IntoParallelCollectorBase<IntoParCollector: ParallelCollector<T>>
{
}
impl<C, T> IntoParallelCollector<T> for C where
    C: IntoParallelCollectorBase<IntoParCollector: ParallelCollector<T>>
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

fn _unindexed_substitutable_to_indexed<T>(x: impl IntoParallelCollector<T>) {
    fn check<T>(_x: impl IntoIndexedParallelCollector<T>) {}
    check::<T>(x);
}
