use super::{Consumer, IntoCollectorBase};

pub struct BasicConsumer<S, SF, MAF, CF, Com, I>
where
    S: Send,
    SF: FnMut(&mut S, usize) -> (S, Com) + Clone + Send,
    MAF: Fn(&S, usize) -> usize + Clone + Send,
    CF: FnOnce(S) -> I + Clone + Send,
    I: IntoCollectorBase<Output: Send>,
    Com: FnOnce(&mut I::Output, I::Output),
{
    pub state: S,
    pub split_f: SF,
    pub ma_f: MAF,
    pub collector_f: CF,
}

impl<S, SF, MAF, CF, Com, I> IntoCollectorBase for BasicConsumer<S, SF, MAF, CF, Com, I>
where
    S: Send,
    SF: FnMut(&mut S, usize) -> (S, Com) + Clone + Send,
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

impl<S, SF, MAF, CF, Com, I> Consumer for BasicConsumer<S, SF, MAF, CF, Com, I>
where
    S: Send,
    SF: FnMut(&mut S, usize) -> (S, Com) + Clone + Send,
    MAF: Fn(&S, usize) -> usize + Clone + Send,
    CF: FnOnce(S) -> I + Clone + Send,
    I: IntoCollectorBase<Output: Send>,
    Com: FnOnce(&mut I::Output, I::Output),
{
    #[inline]
    fn split_off_left_at(
        &mut self,
        index: usize,
    ) -> (
        Self,
        impl FnOnce(&mut Self::Output, Self::Output) + use<S, SF, MAF, CF, Com, I>,
    ) {
        let (state, combiner) = (self.split_f)(&mut self.state, index);
        (
            Self {
                state,
                split_f: self.split_f.clone(),
                ma_f: self.ma_f.clone(),
                collector_f: self.collector_f.clone(),
            },
            combiner,
        )
    }

    #[inline]
    fn max_afford(&self, request: usize) -> usize {
        (self.ma_f)(&self.state, request)
    }
}
