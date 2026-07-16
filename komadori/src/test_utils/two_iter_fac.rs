use std::fmt::Debug;

use proptest::strategy::Strategy;

use super::{DefineItem, TriIteratorFactory};

#[derive(Debug, Clone)]
pub struct TriIterI32Data {
    nums: Vec<i32>,
    unfiltered_till: usize,
}

#[derive(Clone)]
pub struct TriIterI32Factory;

#[derive(Clone)]
pub struct TriIterRefI32Factory;

impl<'d> DefineItem<'d, TriIterI32Data> for TriIterI32Factory {
    type Item = i32;
}

impl TriIteratorFactory<TriIterI32Data> for TriIterI32Factory {
    fn three_iters<'d>(
        &self,
        data: &'d mut TriIterI32Data,
    ) -> [impl Iterator<Item = <Self as DefineItem<'d, TriIterI32Data>>::Item> + use<'d>; 3] {
        [
            data.iter().copied(),
            data.iter().copied(),
            data.iter().copied(),
        ]
    }

    fn one_iter<'d>(
        &self,
        data: &'d mut TriIterI32Data,
    ) -> impl Iterator<Item = <Self as DefineItem<'d, TriIterI32Data>>::Item> + use<'d> {
        data.iter().copied()
    }
}

impl<'d> DefineItem<'d, TriIterI32Data> for TriIterRefI32Factory {
    type Item = &'d i32;
}

impl TriIteratorFactory<TriIterI32Data> for TriIterRefI32Factory {
    fn three_iters<'d>(
        &self,
        data: &'d mut TriIterI32Data,
    ) -> [impl Iterator<Item = <Self as DefineItem<'d, TriIterI32Data>>::Item> + use<'d>; 3] {
        [data.iter(), data.iter(), data.iter()]
    }

    fn one_iter<'d>(
        &self,
        data: &'d mut TriIterI32Data,
    ) -> impl Iterator<Item = <Self as DefineItem<'d, TriIterI32Data>>::Item> + use<'d> {
        data.iter()
    }
}

impl TriIterI32Data {
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
