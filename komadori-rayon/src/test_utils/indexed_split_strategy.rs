use proptest::{
    prelude::{
        prop::{strategy::NewTree, test_runner::TestRunner},
        *,
    },
    strategy::ValueTree,
};

#[derive(Debug, Clone, Default)]
pub enum IndexedSplitDecision {
    #[default]
    Stay,
    Split {
        at: usize,
        left: Box<Self>,
        right: Box<Self>,
    },
}

pub enum IndexedSplitTree {
    Stay {
        len: usize,
    },
    Split {
        at: usize,
        left: Box<Self>,
        right: Box<Self>,
    },
}

#[derive(Debug)]
pub struct IndexedSplitStrategy {
    len: usize,
    max_depth: usize,
}

impl IndexedSplitStrategy {
    pub fn new(len: usize, max_depth: usize) -> Self {
        Self { len, max_depth }
    }

    fn dive_at(&self, at: usize) -> (Self, Self) {
        assert_ne!(self.max_depth, 0);
        assert!(at <= self.len);

        (
            Self {
                len: at,
                max_depth: self.max_depth - 1,
            },
            Self {
                len: self.len - at,
                max_depth: self.max_depth - 1,
            },
        )
    }
}

impl Strategy for IndexedSplitStrategy {
    type Tree = IndexedSplitTree;

    type Value = IndexedSplitDecision;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        Ok(
            if self.max_depth == 0 || runner.rng().random_bool(1.0 / (self.max_depth + 1) as f64) {
                IndexedSplitTree::Stay { len: self.len }
            } else {
                let at = runner.rng().random_range(..=self.len);
                let (left, right) = self.dive_at(at);

                IndexedSplitTree::Split {
                    at,
                    left: left.new_tree(runner)?.into(),
                    right: right.new_tree(runner)?.into(),
                }
            },
        )
    }
}

impl ValueTree for IndexedSplitTree {
    type Value = IndexedSplitDecision;

    fn current(&self) -> Self::Value {
        match self {
            Self::Stay { .. } => IndexedSplitDecision::Stay,
            Self::Split { at, left, right } => IndexedSplitDecision::Split {
                at: *at,
                left: left.current().into(),
                right: right.current().into(),
            },
        }
    }

    fn simplify(&mut self) -> bool {
        // We don't implement simplification and complication for now, until needed.

        false
        // match self {
        //     Self::Stay { .. } => false,
        //     Self::Split { at, left, right } => {
        //         match (&mut **left, &mut **right) {
        //             // Time to shrink the split index
        //             (Self::Stay { len: len_left }, Self::Stay { len: len_right }) => {
        //                 let len = *len_left + *len_right;
        //                 let mid = len.midpoint(0);
        //                 if *at == mid {
        //                     *self = Self::Stay { len };
        //                 } else if *at < mid {
        //                 }
        //             }

        //             (left, Self::Stay { .. }) => {
        //                 left.simplify();
        //             }
        //             (_, right) => {
        //                 right.simplify();
        //             }
        //         }

        //         true
        //     }
        // }
    }

    fn complicate(&mut self) -> bool {
        false
    }
}
