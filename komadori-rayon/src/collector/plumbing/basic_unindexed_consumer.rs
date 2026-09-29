use super::{Consumer, IntoCollectorBase, UnindexedConsumer, impl_split_at_via_unindexed};

pub struct BasicUnindexedConsumer<S, SF, ComF, MAF, CF, Com, I>
where
    S: Send,
    SF: Fn(&S) -> S + Clone + Send,
    ComF: Fn(&S) -> Com + Clone + Send,
    MAF: Fn(&S, usize) -> usize + Clone + Send,
    CF: FnOnce(S) -> I + Clone + Send,
    I: IntoCollectorBase<Output: Send>,
    Com: FnOnce(&mut I::Output, I::Output),
{
    pub state: S,
    pub split_f: SF,
    pub combiner_f: ComF,
    pub ma_f: MAF,
    pub collector_f: CF,
}

impl<S, SF, ComF, MAF, CF, Com, I> IntoCollectorBase
    for BasicUnindexedConsumer<S, SF, ComF, MAF, CF, Com, I>
where
    S: Send,
    SF: Fn(&S) -> S + Clone + Send,
    ComF: Fn(&S) -> Com + Clone + Send,
    MAF: Fn(&S, usize) -> usize + Clone + Send,
    CF: FnOnce(S) -> I + Clone + Send,
    I: IntoCollectorBase<Output: Send>,
    Com: FnOnce(&mut I::Output, I::Output),
{
    type Output = I::Output;

    type IntoCollector = I::IntoCollector;

    #[inline]
    fn into_collector(self) -> Self::IntoCollector {
        (self.collector_f)(self.state).into_collector()
    }
}

impl<S, SF, ComF, MAF, CF, Com, I> Consumer for BasicUnindexedConsumer<S, SF, ComF, MAF, CF, Com, I>
where
    S: Send,
    SF: Fn(&S) -> S + Clone + Send,
    ComF: Fn(&S) -> Com + Clone + Send,
    MAF: Fn(&S, usize) -> usize + Clone + Send,
    CF: FnOnce(S) -> I + Clone + Send,
    I: IntoCollectorBase<Output: Send>,
    Com: FnOnce(&mut I::Output, I::Output),
{
    impl_split_at_via_unindexed!(S, SF, ComF, MAF, CF, Com, I);

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        (self.ma_f)(&self.state, request)
    }
}

impl<S, SF, ComF, MAF, CF, Com, I> UnindexedConsumer
    for BasicUnindexedConsumer<S, SF, ComF, MAF, CF, Com, I>
where
    S: Send,
    SF: Fn(&S) -> S + Clone + Send,
    ComF: Fn(&S) -> Com + Clone + Send,
    MAF: Fn(&S, usize) -> usize + Clone + Send,
    CF: FnOnce(S) -> I + Clone + Send,
    I: IntoCollectorBase<Output: Send>,
    Com: FnOnce(&mut I::Output, I::Output),
{
    #[inline]
    fn split_off_left(&self) -> Self {
        let state = (self.split_f)(&self.state);
        Self {
            state,
            split_f: self.split_f.clone(),
            combiner_f: self.combiner_f.clone(),
            ma_f: self.ma_f.clone(),
            collector_f: self.collector_f.clone(),
        }
    }

    #[inline]
    fn to_combiner(
        &self,
    ) -> impl FnOnce(&mut Self::Output, Self::Output) + use<S, SF, ComF, MAF, CF, Com, I> {
        (self.combiner_f)(&self.state)
    }
}
