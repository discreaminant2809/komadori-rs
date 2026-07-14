use std::fmt::Debug;

use proptest::strategy::Strategy;

use super::{DefineItem, TwoIteratorFactory};

#[derive(Debug, Clone)]
pub struct TwoIterData {
    nums: Vec<i32>,
    unfiltered_till: usize,
}

#[derive(Clone)]
pub struct TwoIterFactory;

#[derive(Clone)]
pub struct TwoIterRefFactory;

impl<'d> DefineItem<'d, TwoIterData> for TwoIterFactory {
    type Item = i32;
}

impl TwoIteratorFactory<TwoIterData> for TwoIterFactory {
    fn two_iters<'d>(
        &self,
        data: &'d mut TwoIterData,
    ) -> [impl Iterator<Item = <Self as DefineItem<'d, TwoIterData>>::Item> + use<'d>; 2] {
        [data.iter().copied(), data.iter().copied()]
    }

    fn one_iter<'d>(
        &self,
        data: &'d mut TwoIterData,
    ) -> impl Iterator<Item = <Self as DefineItem<'d, TwoIterData>>::Item> + use<'d> {
        data.iter().copied()
    }
}

impl<'d> DefineItem<'d, TwoIterData> for TwoIterRefFactory {
    type Item = &'d i32;
}

impl TwoIteratorFactory<TwoIterData> for TwoIterRefFactory {
    fn two_iters<'d>(
        &self,
        data: &'d mut TwoIterData,
    ) -> [impl Iterator<Item = <Self as DefineItem<'d, TwoIterData>>::Item> + use<'d>; 2] {
        [data.iter(), data.iter()]
    }

    fn one_iter<'d>(
        &self,
        data: &'d mut TwoIterData,
    ) -> impl Iterator<Item = <Self as DefineItem<'d, TwoIterData>>::Item> + use<'d> {
        data.iter()
    }
}

impl TwoIterData {
    pub fn strategy() -> impl Strategy<Value = Self> {
        use proptest::{arbitrary::any, collection::vec as propvec};

        (propvec(any::<i32>(), 5), ..=5_usize).prop_map(|(nums, unfiltered_till)| Self {
            nums,
            unfiltered_till,
        })
    }

    fn iter(&self) -> impl Iterator<Item = &i32> {
        let mid = self.unfiltered_till.min(self.nums.len());
        self.nums[..mid]
            .iter()
            .chain(self.nums[mid..].iter().filter(|&&num| num >= 0))
    }
}
