use alloc::{vec, vec::Vec};

use proptest::{
    prelude::{
        Rng,
        prop::{strategy::NewTree, test_runner::TestRunner},
    },
    strategy::{Strategy, ValueTree},
};

#[derive(Debug)]
pub struct FuzzyExecSeqStrategy {
    n: usize,
}

pub struct FuzzyExecSeqTree {
    middles: Vec<MiddleSeqNode>,
    take_middles: usize,
    // The minimum number where `self.middles[..first_meaningful_mid_count]`
    // still collects every of the iterator's item.
    first_meaningful_mid_count: usize,
    end: EndSeqNode,
}

#[derive(Debug, Clone)]
pub struct FuzzyExecSeq {
    pub(super) middles: Vec<MiddleSeqNode>,
    pub(super) end: EndSeqNode,
}

impl FuzzyExecSeq {
    /// Used when we need to isolate a test case.
    #[allow(dead_code)]
    pub fn new(middles: Vec<MiddleSeqNode>, end: EndSeqNode) -> Self {
        Self { middles, end }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MiddleSeqNode {
    Reserve { additional: usize },
    MaxAfford { request: usize },
    Collect,
    AssumeReservedCollect,
    CollectMany { n: usize },
}

#[derive(Debug, Clone, Copy)]
pub enum EndSeqNode {
    Finish,
    FinishBoxed,
    CollectThenFinish,
}

impl Strategy for FuzzyExecSeqStrategy {
    type Tree = FuzzyExecSeqTree;

    type Value = FuzzyExecSeq;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        Ok(self.new_tree_impl(runner))
    }
}

impl FuzzyExecSeqStrategy {
    pub fn new(n: usize) -> Self {
        Self { n }
    }

    fn new_tree_impl(&self, runner: &mut TestRunner) -> FuzzyExecSeqTree {
        let mut middles = vec![];
        let mut remaining = self.n;
        let mut reserved_amount = 0;
        let mut exccessive = false;
        let mut first_meaningful_mid_count = 0;
        let rng = runner.rng();

        while remaining > 0 && rng.random_bool(0.75) {
            let seq = match (rng.random_range(0..=4), reserved_amount) {
                (0, _) => {
                    reserved_amount = rng.random_range(..=remaining.saturating_add(remaining / 3));
                    MiddleSeqNode::Reserve {
                        additional: reserved_amount,
                    }
                }
                (1, _) => MiddleSeqNode::MaxAfford {
                    request: if rng.random_bool(0.8) {
                        remaining.saturating_add(2)
                    } else {
                        // In a small chance, we request an arbitrarily large (hopefully)
                        // number to see if an infinite collector still return correctly.
                        rng.random::<u64>() as usize
                    },
                },
                (2, _) | (3, 0) => {
                    remaining -= 1;
                    reserved_amount = reserved_amount.saturating_sub(1);
                    first_meaningful_mid_count = middles.len() + 1;
                    MiddleSeqNode::Collect
                }
                (3, 1..) => {
                    remaining -= 1;
                    reserved_amount -= 1;
                    first_meaningful_mid_count = middles.len() + 1;
                    MiddleSeqNode::AssumeReservedCollect
                }
                (4, _) => {
                    reserved_amount = 0;
                    let n = rng.random_range(..=remaining.saturating_add(remaining / 3));
                    if remaining < n {
                        remaining = 0;
                        exccessive = true;
                    } else {
                        remaining -= n;
                    }
                    first_meaningful_mid_count = middles.len() + 1;
                    MiddleSeqNode::CollectMany { n }
                }
                _ => unreachable!("impossible random value for 0..=4 range"),
            };

            middles.push(seq);
        }

        let end = if remaining > 0 {
            EndSeqNode::CollectThenFinish
        } else if exccessive {
            EndSeqNode::Finish
        } else {
            match rng.random_range(0..=3) {
                0 => EndSeqNode::Finish,
                1 => EndSeqNode::FinishBoxed,
                2 => EndSeqNode::CollectThenFinish,
                3 => {
                    middles.push(MiddleSeqNode::CollectMany { n: self.n / 3 });
                    EndSeqNode::Finish
                }
                _ => unreachable!("impossible random value for 0..=3 range"),
            }
        };

        FuzzyExecSeqTree {
            take_middles: middles.len(),
            middles,
            first_meaningful_mid_count,
            end,
        }
    }
}

impl ValueTree for FuzzyExecSeqTree {
    type Value = FuzzyExecSeq;

    fn current(&self) -> Self::Value {
        Self::Value {
            middles: self.middles[..self.take_middles].to_vec(),
            end: if self.take_middles < self.first_meaningful_mid_count {
                EndSeqNode::CollectThenFinish
            } else {
                self.end
            },
        }
    }

    fn simplify(&mut self) -> bool {
        if self.take_middles == 0 {
            false
        } else {
            self.take_middles -= 1;
            true
        }
    }

    fn complicate(&mut self) -> bool {
        if self.take_middles >= self.middles.len() {
            false
        } else {
            self.take_middles += 1;
            true
        }
    }
}
