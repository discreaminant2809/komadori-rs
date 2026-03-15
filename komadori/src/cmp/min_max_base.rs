use std::{fmt::Debug, ops::ControlFlow};

use itertools::MinMaxResult;

use crate::collector::{Collector, CollectorBase};

use super::Comparator;

#[derive(Clone)]
pub struct MinMaxBase<T, Cmp> {
    state: State<T>,
    cmp: Cmp,
}

#[derive(Debug, Clone)]
enum State<T> {
    NoElements,
    OneElement(T),
    MinMax { min: T, max: T, prev: Option<T> },
}

impl<T, Cmp> MinMaxBase<T, Cmp> {
    #[inline]
    pub const fn new(cmp: Cmp) -> Self {
        Self {
            state: State::NoElements,
            cmp,
        }
    }

    pub fn debug_state(&self) -> &impl Debug
    where
        T: Debug,
    {
        &self.state
    }
}

impl<T, Cmp> CollectorBase for MinMaxBase<T, Cmp>
where
    Cmp: Comparator<T>,
{
    type Output = MinMaxResult<T>;

    fn finish(mut self) -> Self::Output {
        match self.state {
            State::NoElements => MinMaxResult::NoElements,
            State::OneElement(item) => MinMaxResult::OneElement(item),
            State::MinMax {
                min,
                max,
                prev: Some(prev),
            } if self.cmp.lt(&prev, &min) => MinMaxResult::MinMax(prev, max),
            State::MinMax {
                min,
                max,
                prev: Some(prev),
            } if self.cmp.le(&max, &prev) => MinMaxResult::MinMax(min, prev),
            State::MinMax { min, max, .. } => MinMaxResult::MinMax(min, max),
        }
    }
}

impl<T, Cmp> Collector<T> for MinMaxBase<T, Cmp>
where
    Cmp: Comparator<T>,
{
    #[inline]
    fn collect(&mut self, item: T) -> ControlFlow<()> {
        match &mut self.state {
            State::NoElements => self.state = State::OneElement(item),
            State::OneElement(_) => {
                let State::OneElement(prev) = std::mem::replace(&mut self.state, State::NoElements)
                else {
                    unreachable!("the state is somehow incorrect");
                };

                if self.cmp.lt(&item, &prev) {
                    self.state = State::MinMax {
                        min: item,
                        max: prev,
                        prev: None,
                    }
                } else {
                    self.state = State::MinMax {
                        min: prev,
                        max: item,
                        prev: None,
                    }
                }
            }
            State::MinMax { min, max, prev } => {
                let Some(prev) = prev.take() else {
                    *prev = Some(item);
                    return ControlFlow::Continue(());
                };

                if self.cmp.lt(&item, &prev) {
                    self.cmp.min_assign(min, item);
                    self.cmp.max_assign(max, prev);
                } else {
                    self.cmp.min_assign(min, prev);
                    self.cmp.max_assign(max, item);
                }
            }
        }

        ControlFlow::Continue(())
    }

    #[inline]
    fn collect_many(&mut self, items: impl IntoIterator<Item = T>) -> ControlFlow<()> {
        let mut items = items.into_iter();

        'outer: loop {
            match &mut self.state {
                State::NoElements => {
                    self.state = {
                        let Some(item) = items.next() else {
                            break;
                        };
                        State::OneElement(item)
                    }
                }
                State::OneElement(_) => {
                    let Some(item) = items.next() else {
                        break;
                    };

                    let State::OneElement(prev) =
                        std::mem::replace(&mut self.state, State::NoElements)
                    else {
                        unreachable!("the state is somehow incorrect");
                    };

                    if self.cmp.lt(&item, &prev) {
                        self.state = State::MinMax {
                            min: item,
                            max: prev,
                            prev: None,
                        }
                    } else {
                        self.state = State::MinMax {
                            min: prev,
                            max: item,
                            prev: None,
                        }
                    }
                }
                State::MinMax { min, max, prev } => {
                    let Some(mut first) = prev.take().or_else(|| items.next()) else {
                        break;
                    };

                    let Some(mut second) = items.next() else {
                        *prev = Some(first);
                        break;
                    };

                    loop {
                        if self.cmp.lt(&second, &first) {
                            self.cmp.min_assign(min, second);
                            self.cmp.max_assign(max, first);
                        } else {
                            self.cmp.min_assign(min, first);
                            self.cmp.max_assign(max, second);
                        }

                        match items.next() {
                            Some(item) => first = item,
                            None => break 'outer,
                        }

                        match items.next() {
                            Some(item) => second = item,
                            None => {
                                *prev = Some(first);
                                break 'outer;
                            }
                        }
                    }
                }
            }
        }

        ControlFlow::Continue(())
    }

    fn collect_then_finish(mut self, items: impl IntoIterator<Item = T>) -> Self::Output {
        let mut items = items.into_iter();

        'outer: loop {
            match self.state {
                State::NoElements => {
                    self.state = {
                        let Some(item) = items.next() else {
                            break MinMaxResult::NoElements;
                        };
                        State::OneElement(item)
                    }
                }
                State::OneElement(prev) => {
                    let Some(item) = items.next() else {
                        break MinMaxResult::OneElement(prev);
                    };

                    if self.cmp.lt(&item, &prev) {
                        self.state = State::MinMax {
                            min: item,
                            max: prev,
                            prev: None,
                        }
                    } else {
                        self.state = State::MinMax {
                            min: prev,
                            max: item,
                            prev: None,
                        }
                    }
                }
                State::MinMax {
                    mut min,
                    mut max,
                    prev,
                } => {
                    let Some(mut first) = prev.or_else(|| items.next()) else {
                        break MinMaxResult::MinMax(min, max);
                    };

                    let Some(mut second) = items.next() else {
                        break if self.cmp.lt(&first, &min) {
                            MinMaxResult::MinMax(first, max)
                        } else if self.cmp.le(&max, &first) {
                            MinMaxResult::MinMax(min, first)
                        } else {
                            MinMaxResult::MinMax(min, max)
                        };
                    };

                    loop {
                        if self.cmp.lt(&second, &first) {
                            self.cmp.min_assign(&mut min, second);
                            self.cmp.max_assign(&mut max, first);
                        } else {
                            self.cmp.min_assign(&mut min, first);
                            self.cmp.max_assign(&mut max, second);
                        }

                        match items.next() {
                            Some(item) => first = item,
                            None => break 'outer MinMaxResult::MinMax(min, max),
                        }

                        match items.next() {
                            Some(item) => second = item,
                            None => {
                                break 'outer if self.cmp.lt(&first, &min) {
                                    MinMaxResult::MinMax(first, max)
                                } else if self.cmp.le(&max, &first) {
                                    MinMaxResult::MinMax(min, first)
                                } else {
                                    MinMaxResult::MinMax(min, max)
                                };
                            }
                        }
                    }
                }
            }
        }
    }
}
