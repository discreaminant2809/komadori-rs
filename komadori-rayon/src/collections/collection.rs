pub trait CollectionBase {
    fn len(&self) -> usize;

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// If the collection's capacity doesn't matter (e.g. `LinkedList`, `BTreeMap`),
    /// returns `0`.
    fn capacity(&self) -> usize;

    fn reserve(&mut self, additional: usize) {
        let _ = additional;
    }

    fn append_many(&mut self, chunks: impl IntoIterator<Item = Self>, mut append_len: usize)
    where
        Self: Append + Sized,
    {
        let mut chunks = chunks.into_iter().fuse();

        // We seek for a better buffer when there's no element in the original buffer.
        if self.is_empty() {
            for chunk in &mut chunks {
                if !chunk.is_empty() {
                    // Chunk has some elements. We must use it. No question asked!
                    *self = chunk;

                    // Shouldn't panic. The reduction of linked vec is assumed to
                    // correctly report the amount to append.
                    append_len -= self.len();
                    break;
                }

                // There's a case that none of the chunks have elements at all.
                // In this case, we simply continuously seek for the better capacity,
                // so that the caller ends up with having a bigger buffer
                // to work with!
                if self.capacity() < chunk.capacity() {
                    *self = chunk;
                }
            }
        }
        // If the length isn't 0,
        // we have no other choice but to obey the original buffer
        // which already contains some items.

        self.reserve(append_len);
        for chunk in chunks {
            self.append(chunk);
        }
    }
}

pub trait Collection<T>: CollectionBase {
    fn push(&mut self, item: T);

    unsafe fn assume_reserved_push(&mut self, item: T) {
        self.push(item);
    }

    fn push_many(&mut self, items: impl IntoIterator<Item = T>);
}

pub trait Append<Other = Self>: CollectionBase {
    fn append(&mut self, other: Other);
}

impl<C> CollectionBase for &mut C
where
    C: CollectionBase + ?Sized,
{
    #[inline]
    fn len(&self) -> usize {
        C::len(self)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        C::is_empty(self)
    }

    #[inline]
    fn capacity(&self) -> usize {
        C::capacity(self)
    }

    #[inline]
    fn reserve(&mut self, additional: usize) {
        C::reserve(self, additional);
    }
}

impl<C, OC> Append<OC> for &mut C
where
    C: Append<OC> + ?Sized,
{
    #[inline]
    fn append(&mut self, other: OC) {
        C::append(self, other);
    }
}

impl<C, T> Collection<T> for &mut C
where
    C: Collection<T> + ?Sized,
{
    #[inline]
    fn push(&mut self, item: T) {
        C::push(self, item);
    }

    #[inline]
    unsafe fn assume_reserved_push(&mut self, item: T) {
        unsafe {
            C::assume_reserved_push(self, item);
        }
    }

    #[inline]
    fn push_many(&mut self, items: impl IntoIterator<Item = T>) {
        C::push_many(self, items);
    }
}
