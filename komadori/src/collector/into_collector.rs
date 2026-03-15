use super::{Collector, CollectorBase};

/// Conversion into a [`Collector`].
///
/// By implementing this trait for a type, you define how it will be converted to a collector.
///
/// # Usage in trait bounds
///
/// Using `IntoCollectorBase` in trait bounds allows a function to be generic over both
/// [`CollectorBase`] and `IntoCollectorBase`.
/// This is convenient for users of the function, so when they are using it
/// they do not have to make an extra call to
/// [`IntoCollectorBase::into_collector()`] to obtain an instance of [`Collector`].
///
/// Prefer [`IntoCollector`] whenever possible. [`IntoCollector`] can specify
/// the item type more easily, instead of writing
/// `C: IntoCollectorBase<IntoCollector: Collector<T>>`.
pub trait IntoCollectorBase {
    /// The output of the collector.
    type Output;

    /// Which collector being produced?
    type IntoCollector: CollectorBase<Output = Self::Output>;

    /// Creates a collector from a value.
    fn into_collector(self) -> Self::IntoCollector;
}

/// A trait alias for [`IntoCollectorBase`] types that has its collector
/// implement [`Collector`].
///
/// This trait is automatically implemented for such types.
///
/// Users generally should prefer this bound if they want to specify the
/// item type they need, instead of writing
/// `C: IntoCollectorBase<IntoCollector: Collector<T>>`.
pub trait IntoCollector<T>: IntoCollectorBase<IntoCollector: Collector<T>> {}

impl<C> IntoCollectorBase for C
where
    C: CollectorBase,
{
    type Output = C::Output;

    type IntoCollector = C;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        self
    }
}

impl<C, T> IntoCollector<T> for C where C: IntoCollectorBase<IntoCollector: Collector<T>> + ?Sized {}
