use std::fmt::Debug;

use proptest::strategy::Strategy;

use super::{DefineItem, TwoIteratorFactory};

#[derive(Debug, Clone)]
pub struct TwoIterMutData {
    nums: Vec<i32>,
    nums_for_model: Vec<i32>,
    unfiltered_till: usize,
}

#[derive(Clone)]
pub struct TwoIterMutFactory;

impl<'d> DefineItem<'d, TwoIterMutData> for TwoIterMutFactory {
    type Item = &'d mut i32;
}

impl TwoIteratorFactory<TwoIterMutData> for TwoIterMutFactory {
    fn two_iters<'d>(
        &self,
        data: &'d mut TwoIterMutData,
    ) -> [impl Iterator<Item = <Self as DefineItem<'d, TwoIterMutData>>::Item> + use<'d>; 2] {
        data.two_iters_mut()
    }

    fn one_iter<'d>(
        &self,
        data: &'d mut TwoIterMutData,
    ) -> impl Iterator<Item = <Self as DefineItem<'d, TwoIterMutData>>::Item> + use<'d> {
        let [iter, _] = data.two_iters_mut();
        iter
    }
}

impl TwoIterMutData {
    pub fn strategy() -> impl Strategy<Value = Self> {
        use proptest::{arbitrary::any, collection::vec as propvec};

        (propvec(any::<i32>(), 5), ..=5_usize).prop_map(|(nums, unfiltered_till)| Self {
            nums_for_model: nums.clone(),
            nums,
            unfiltered_till,
        })
    }

    fn two_iters_mut(&mut self) -> [impl Iterator<Item = &mut i32>; 2] {
        let mid = self.unfiltered_till.min(self.nums.len());

        fn iter_mut(nums: &mut [i32], mid: usize) -> impl Iterator<Item = &mut i32> {
            let (first, second) = nums.split_at_mut(mid);
            first
                .iter_mut()
                .chain(second.iter_mut().filter(|&&mut num| num >= 0))
        }

        [
            iter_mut(&mut self.nums, mid),
            iter_mut(&mut self.nums_for_model, mid),
        ]
    }
}
