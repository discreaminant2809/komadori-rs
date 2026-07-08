/// Represents `(usize, Option<usize>)` returned by [`Iterator::size_hint()`].
///
/// It contains various methods to handle the size hint easier.
#[derive(Debug, Default, Clone, Copy)]
pub struct SizeHint {
    lower: usize,
    upper: Option<usize>,
}

impl From<(usize, Option<usize>)> for SizeHint {
    #[inline]
    fn from((lower, upper): (usize, Option<usize>)) -> Self {
        debug_assert!(
            upper.is_none_or(|upper| upper >= lower),
            "{:?} is an incorrect size hint",
            (lower, upper)
        );
        Self { lower, upper }
    }
}

impl SizeHint {
    #[inline]
    pub fn from_iter(iter: &impl Iterator) -> Self {
        Self::from(iter.size_hint())
    }

    #[inline]
    pub fn lower(self) -> usize {
        self.lower
    }

    #[inline]
    pub fn exact_size(self) -> Option<usize> {
        self.upper.filter(|&upper| upper == self.lower)
    }
}
