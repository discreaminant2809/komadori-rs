mod map_collector;

pub use map_collector::MapCollector;

use super::{CollectorBase, Consumer, IntoCollectorBase, UnindexedConsumer};

pub trait ConsumerExt: Consumer {
    /// Useful for adapters where most of the time the serial collector
    /// is the only thing transformed.
    fn map_collector<F, C>(self, f: F) -> MapCollector<Self, F>
    where
        F: FnOnce(Self::IntoCollector) -> C + Clone + Send,
        C: CollectorBase<Output = Self::Output>,
    {
        MapCollector::new(self, f)
    }
}

impl<C> ConsumerExt for C where C: Consumer {}
