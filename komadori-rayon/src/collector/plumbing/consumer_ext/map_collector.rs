use super::{CollectorBase, Consumer, IntoCollectorBase, UnindexedConsumer};

pub struct MapCollector<C, F> {
    consumer: C,
    f: F,
}

impl<C, F> MapCollector<C, F> {
    pub(super) fn new(consumer: C, f: F) -> Self {
        Self { consumer, f }
    }
}

impl<C, F, Collector> IntoCollectorBase for MapCollector<C, F>
where
    C: IntoCollectorBase,
    F: FnOnce(C::IntoCollector) -> Collector,
    Collector: CollectorBase<Output = C::Output>,
{
    type Output = C::Output;

    type IntoCollector = Collector;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        (self.f)(self.consumer.into_collector())
    }
}

impl<C, F, Collector> Consumer for MapCollector<C, F>
where
    C: Consumer,
    F: FnOnce(C::IntoCollector) -> Collector + Clone + Send,
    Collector: CollectorBase<Output = C::Output>,
{
    #[inline]
    fn split_off_left_at(
        &mut self,
        index: usize,
    ) -> (
        Self,
        impl FnOnce(&mut Self::Output, Self::Output) + use<C, F, Collector>,
    ) {
        let (consumer, combiner) = self.consumer.split_off_left_at(index);
        (
            Self {
                consumer,
                f: self.f.clone(),
            },
            combiner,
        )
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        self.consumer.max_afford(request)
    }
}

impl<C, F, Collector> UnindexedConsumer for MapCollector<C, F>
where
    C: UnindexedConsumer,
    F: FnOnce(C::IntoCollector) -> Collector + Clone + Send,
    Collector: CollectorBase<Output = C::Output>,
{
    #[inline]
    fn split_off_left(&self) -> Self {
        Self {
            consumer: self.consumer.split_off_left(),
            f: self.f.clone(),
        }
    }

    #[inline]
    fn to_combiner(&self) -> impl FnOnce(&mut Self::Output, Self::Output) + use<C, F, Collector> {
        self.consumer.to_combiner()
    }
}
