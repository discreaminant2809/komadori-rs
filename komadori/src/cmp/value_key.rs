use std::cmp::Ordering;

/// A helper struct for `max_by_key` and `min_by_key`
///
/// It will ONLY compare keys of the two instances.
#[derive(Debug, Clone)]
pub struct ValueKey<T, K> {
    value: T,
    key: K,
}

impl<T, K> ValueKey<T, K> {
    #[inline]
    pub fn new(value: T, f: impl FnOnce(&T) -> K) -> Self {
        let key = f(&value);
        Self { value, key }
    }

    #[inline]
    pub fn into_value(self) -> T {
        self.value
    }
}

impl<T, K: PartialEq> PartialEq for ValueKey<T, K> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T, K: Eq> Eq for ValueKey<T, K> {}

impl<T, K: PartialOrd> PartialOrd for ValueKey<T, K> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.key.partial_cmp(&other.key)
    }
}

impl<T, K: Ord> Ord for ValueKey<T, K> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}
