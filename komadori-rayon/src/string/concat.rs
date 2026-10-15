use std::{borrow::Borrow, collections::LinkedList};

use crate::slice::{OwnedSlice, OwnedSliceBase, ParConcat, ParConcatSealed};

impl ParConcat for String {}

impl ParConcatSealed for String {}

impl OwnedSliceBase for String {
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

    fn append(&mut self, other: Self) {
        *self += &other;
    }

    fn append_from_linked_list(&mut self, others: LinkedList<Self>, len: usize) {
        super::append_linked_string(self, others, len);
    }
}

impl<T> OwnedSlice<T> for String
where
    T: Borrow<str>,
{
    #[inline]
    fn push(&mut self, item: T) {
        *self += item.borrow();
    }
}

#[cfg(test)]
mod proptests {
    use proptest::string::string_regex;

    use crate::{slice::ParConcat, test_utils::prelude::*};

    macro_rules! concat_test {
        ($test_macro:ident, $test_name:ident, $method_name:ident) => {
            $test_macro!($test_name {
                iter_data: {
                    let mut strs = propvec(string_regex(".{0,8}").unwrap(), ..=5);
                },
                other_data: {
                    let mut starting_str = string_regex(".{0,4}").unwrap();
                },
                iter: strs.par_iter().cloned(),
                collector: starting_str.$method_name(),
                starting_ma_f: |request| request,
                expected_f: |iter, _, request| {
                    let mut res = starting_str.clone();
                    res.extend(iter);
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
