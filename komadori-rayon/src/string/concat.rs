use std::borrow::Borrow;

use crate::slice::{OwnedSlice, ParConcat, ParConcatSealed};

impl ParConcat for String {}

impl ParConcatSealed for String {}

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
