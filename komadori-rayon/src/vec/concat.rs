use std::{borrow::Borrow, collections::LinkedList};

use crate::slice::{OwnedSlice, OwnedSliceBase, ParConcat, ParConcatSealed};

impl<T> ParConcat for Vec<T> {}

impl<T> ParConcatSealed for Vec<T> {}

impl<T> OwnedSliceBase for Vec<T> {
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

    fn append(&mut self, mut other: Self) {
        self.append(&mut other);
    }

    fn append_from_linked_list(&mut self, others: LinkedList<Self>, len: usize) {
        super::append_linked_vec(self, others, len);
    }
}

impl<Item, T> OwnedSlice<Item> for Vec<T>
where
    Item: Borrow<[T]>,
    T: Clone,
{
    #[inline]
    fn push(&mut self, item: Item) {
        self.extend_from_slice(item.borrow());
    }
}

#[cfg(test)]
mod proptests {
    use crate::{slice::ParConcat, test_utils::prelude::*};

    macro_rules! concat_test {
        ($test_macro:ident, $test_name:ident, $method_name:ident) => {
            $test_macro!($test_name {
                iter_data: {
                    let mut nums = propvec(propvec(any::<i32>(), ..=2), ..=5);
                },
                other_data: {
                    let mut starting_nums = propvec(any::<i32>(), ..=2);
                },
                iter: nums.par_iter().cloned(),
                collector: starting_nums.$method_name(),
                starting_ma_f: |request| request,
                expected_f: |iter, _, request| {
                    let mut res = starting_nums.clone();
                    for slice in iter {
                        res.extend(slice);
                    }
                    (res, Continue(((), request)))
                },
                output_pred: |actual, expected| actual[..] == expected[..],
                state_pred: state_is_irrelevant(),
            });
        };
    }

    concat_test!(par_collector_test, into_par_concat_indexed, into_par_concat);

    concat_test!(par_collector_test, par_concat_mut_indexed, par_concat_mut);

    concat_test!(
        unindexed_par_collector_test,
        into_par_concat_unindexed,
        into_par_concat
    );

    concat_test!(
        unindexed_par_collector_test,
        par_concat_mut_unindexed,
        par_concat_mut
    );
}
