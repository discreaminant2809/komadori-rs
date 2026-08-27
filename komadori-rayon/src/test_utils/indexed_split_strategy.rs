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

impl IndexedSplitTree {
    fn deepest_depth(&self) -> usize {
        match self {
            Self::Stay { .. } => 0,
            Self::Split { left, right, .. } => 1 + left.deepest_depth().max(right.deepest_depth()),
        }
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
        match self {
            // Already the most simplified.
            Self::Stay { .. } => false,

            Self::Split { left, right, .. } => {
                let (left, right) = (&mut **left, &mut **right);

                match (left, right) {
                    (Self::Stay { .. }, right @ Self::Split { .. }) => assert!(right.simplify()),
                    (left @ Self::Split { .. }, Self::Stay { .. }) => assert!(left.simplify()),
                    (Self::Stay { len: left_len }, Self::Stay { len: right_len }) => {
                        *self = Self::Stay {
                            len: *left_len + *right_len,
                        }
                    }

                    // We strive for a balance tree first.
                    // O(max_depth^2)
                    // But the max_depth isn't gonna be large (about <= 4) anyway.
                    // The approach of caching the max_depth would be very complicated.
                    (left, right) if left.deepest_depth() < right.deepest_depth() => {
                        assert!(right.simplify())
                    }
                    (left, _right) => assert!(left.simplify()),
                }

                true
            }
        }
    }

    fn complicate(&mut self) -> bool {
        false
    }
}
