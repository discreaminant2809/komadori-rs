use super::{
    ParallelCollector, ParallelCollectorBase, UnindexedParallelCollector,
    UnindexedParallelCollectorBase,
};

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
pub trait IntoParallelCollector<T>:
    IntoParallelCollectorBase<IntoParCollector: ParallelCollector<T>>
{
}
impl<C, T> IntoParallelCollector<T> for C where
    C: IntoParallelCollectorBase<IntoParCollector: ParallelCollector<T>>
{
}

///
pub trait IntoUnindexedParallelCollectorBase:
    IntoParallelCollectorBase<IntoParCollector: UnindexedParallelCollectorBase>
{
}
impl<C> IntoUnindexedParallelCollectorBase for C where
    C: IntoParallelCollectorBase<IntoParCollector: UnindexedParallelCollectorBase>
{
}

///
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
