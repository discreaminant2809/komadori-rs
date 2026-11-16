use crate::collections::{Append, Collection, CollectionBase};

impl<T> CollectionBase for Vec<T> {
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.capacity()
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        self.reserve(additional);
    }
}

impl<T> Append for Vec<T> {
    #[inline]
    fn append(&mut self, mut other: Self) {
        self.append(&mut other);
    }
}

impl<T> Collection<T> for Vec<T> {
    #[inline]
    fn push(&mut self, item: T) {
        self.push(item);
    }

    #[inline]
    unsafe fn assume_reserved_push(&mut self, item: T) {
        unsafe {
            super::push_unchecked(self, item);
        }
    }

    #[inline]
    fn push_many(&mut self, items: impl IntoIterator<Item = T>) {
        self.extend(items);
    }
}

impl<'a, T> Collection<&'a T> for Vec<T>
where
    T: Copy,
{
    #[inline]
    fn push(&mut self, &item: &'a T) {
        self.push(item);
    }

    #[inline]
    unsafe fn assume_reserved_push(&mut self, &item: &'a T) {
        unsafe {
            super::push_unchecked(self, item);
        }
    }

    #[inline]
    fn push_many(&mut self, items: impl IntoIterator<Item = &'a T>) {
        self.extend(items);
    }
}

impl<'a, T> Collection<&'a mut T> for Vec<T>
where
    T: Copy,
{
    #[inline]
    fn push(&mut self, &mut item: &'a mut T) {
        self.push(item);
    }

    #[inline]
    unsafe fn assume_reserved_push(&mut self, &mut item: &'a mut T) {
        unsafe {
            super::push_unchecked(self, item);
        }
    }

    #[inline]
    fn push_many(&mut self, items: impl IntoIterator<Item = &'a mut T>) {
        self.extend(items.into_iter().map(|&mut item| item));
    }
}
