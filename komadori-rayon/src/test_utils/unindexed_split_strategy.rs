use proptest::{
    prelude::{
        prop::{strategy::NewTree, test_runner::TestRunner},
        *,
    },
    strategy::ValueTree,
};

#[derive(Debug, Clone, Default)]
pub enum UnindexedSplitDecision {
    #[default]
    Stay,
    Split {
        left: Box<Self>,
        right: Box<Self>,
    },
}

#[derive(Debug, Default)]
pub struct UnindexedSplitStrategy {
    max_depth: usize,
}

pub struct UnindexedSplitTree {
    decision: UnindexedSplitDecision,
}

impl UnindexedSplitDecision {
    pub fn deepest_depth(&self) -> usize {
        match self {
            UnindexedSplitDecision::Stay => 0,
            UnindexedSplitDecision::Split { left, right } => {
                1 + left.deepest_depth().max(right.deepest_depth())
            }
        }
    }

    fn simplify(&mut self) -> bool {
        match self {
            // Already the most simplified.
            Self::Stay => false,

            Self::Split { left, right } => {
                // We strive for a balance tree first.
                // O(max_depth^2)
                // But the max_depth isn't gonna be large (about <= 4) anyway.
                // The approach of caching the max_depth would be very complicated.
                let left_depth = left.deepest_depth();
                let right_depth = right.deepest_depth();

                if left_depth < right_depth {
                    // Always simplifiable if the depth is positive.
                    assert!(right.simplify());
                } else if left_depth > right_depth {
                    assert!(left.simplify());
                } else if left_depth == 0 {
                    *self = Self::Stay;
                } else {
                    assert!(right.simplify());
                }

                true
            }
        }
    }

    fn complicate(&mut self) -> bool {
        false
        // match self {
        //     Self::Stay => {
        //         *self = Self::Split {
        //             left: Self::Stay.into(),
        //             right: Self::Stay.into(),
        //         }
        //     }
        //     Self::Split { left, right } => {
        //         left.complicate();
        //         right.complicate();
        //     }
        // }

        // true
    }
}

impl UnindexedSplitStrategy {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    fn dive(&self) -> Self {
        assert_ne!(self.max_depth, 0);

        Self {
            max_depth: self.max_depth - 1,
        }
    }
}

impl Strategy for UnindexedSplitStrategy {
    type Tree = UnindexedSplitTree;

    type Value = UnindexedSplitDecision;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        Ok(
            if self.max_depth == 0 || runner.rng().random_bool(1.0 / (self.max_depth + 1) as f64) {
                UnindexedSplitTree {
                    decision: UnindexedSplitDecision::Stay,
                }
            } else {
                UnindexedSplitTree {
                    decision: UnindexedSplitDecision::Split {
                        left: self.dive().new_tree(runner)?.decision.into(),
                        right: self.dive().new_tree(runner)?.decision.into(),
                    },
                }
            },
        )
    }
}

impl ValueTree for UnindexedSplitTree {
    type Value = UnindexedSplitDecision;

    fn current(&self) -> Self::Value {
        self.decision.clone()
    }

    fn simplify(&mut self) -> bool {
        self.decision.simplify()
    }

    fn complicate(&mut self) -> bool {
        self.decision.complicate()
    }
}
