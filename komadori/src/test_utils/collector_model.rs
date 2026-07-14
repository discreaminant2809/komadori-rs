use std::{fmt::Debug, ops::ControlFlow};

pub trait CollectorModel<T, EO: Debug, AO: ?Sized> {
    fn advance(&mut self, item: T);

    fn expected_max_afford(&self, request: usize) -> usize;

    fn expected_cf(&self) -> ControlFlow<()>;

    // We don't use `Eq` because we don't always (but mostly) test for equality.
    fn into_expected_output_and_pred(
        self,
    ) -> (
        EO,
        // So that we can use a different output type than the collector
        impl FnOnce(&EO, &AO) -> bool,
    );
}

pub struct BasicCollectorModel<S, AF, MAF, CFF, OPF>
where
    MAF: Fn(&S, usize) -> usize,
    CFF: Fn(&S) -> ControlFlow<()>,
{
    pub state: S,
    pub advance_f: AF,
    pub max_afford_f: MAF,
    pub cf_f: CFF,
    pub output_and_pred_f: OPF,
}

impl<S, AF, MAF, CFF, OPF, EO, AO, P, T> CollectorModel<T, EO, AO>
    for BasicCollectorModel<S, AF, MAF, CFF, OPF>
where
    AF: FnMut(&mut S, T),
    MAF: Fn(&S, usize) -> usize,
    CFF: Fn(&S) -> ControlFlow<()>,
    OPF: FnOnce(S) -> (EO, P),
    P: FnOnce(&EO, &AO) -> bool,
    EO: Debug,
    AO: ?Sized,
{
    fn advance(&mut self, item: T) {
        (self.advance_f)(&mut self.state, item);
    }

    fn expected_max_afford(&self, request: usize) -> usize {
        (self.max_afford_f)(&self.state, request)
    }

    fn expected_cf(&self) -> ControlFlow<()> {
        (self.cf_f)(&self.state)
    }

    fn into_expected_output_and_pred(
        self,
    ) -> (
        EO,
        // So that we can use a different output type than the collector
        impl FnOnce(&EO, &AO) -> bool,
    ) {
        (self.output_and_pred_f)(self.state)
    }
}
