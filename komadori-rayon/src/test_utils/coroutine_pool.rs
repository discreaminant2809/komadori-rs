use std::{
    cell::RefCell,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use komadori::prelude::*;
use proptest::prelude::*;
use rand::{RngExt, SeedableRng, distr::Distribution, rngs::Xoshiro128PlusPlus};

use crate::{
    collector::plumbing::{Combiner, Consumer, UnindexedConsumer},
    test_utils::{IndexedProducer, IndexedSplitDecision},
};

use super::{Producer, UnindexedSplitDecision};

#[derive(Debug)]
pub struct CoroutinePool {
    rng: Xoshiro128PlusPlus,
}

// pub enum Event {
//     StartBridging,
//     StayCreateSerialCollector,
//     StayUsingSerialCollector,
//     StayReturn,
// }

type Work<'a> = Pin<Box<dyn Future<Output = ()> + 'a>>;
type Queue<'a> = Vec<Work<'a>>;
type RcrcSharedState<'a> = Rc<RefCell<SharedState<'a>>>;

struct SharedState<'a> {
    queue: Queue<'a>,
    rng: Xoshiro128PlusPlus,
    // task_pick_log: Vec<usize>,
    // event_log: Vec<Event>,
}

impl CoroutinePool {
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: Xoshiro128PlusPlus::seed_from_u64(seed),
        }
    }

    pub fn prop() -> impl Strategy<Value = CoroutinePool> {
        prop::arbitrary::any::<u64>()
            .prop_map(CoroutinePool::with_seed)
            // Because even a slight change in seed results in a completely different execution,
            // hence shinking is meaningless.
            .no_shrink()
    }

    pub fn bridge_unindexed<P, C>(
        &mut self,
        producer: P,
        consumer: C,
        split_decision: &UnindexedSplitDecision,
    ) -> C::Output
    where
        P: Producer,
        C: UnindexedConsumer<IntoCollector: Collector<P::Item>>,
    {
        struct UnindexedJob<'a, P, C> {
            producer: P,
            consumer: C,
            split_decision: &'a UnindexedSplitDecision,
        }

        impl<P, C> Job for UnindexedJob<'_, P, C>
        where
            P: Producer,
            C: UnindexedConsumer<IntoCollector: Collector<P::Item>>,
        {
            type Output = C::Output;

            fn execute<'a>(self, state: RcrcSharedState<'a>) -> impl Future<Output = Self::Output>
            where
                Self: 'a,
            {
                bridge_task(state, self.producer, self.consumer, self.split_decision)
            }
        }

        self.spawn(UnindexedJob {
            producer,
            consumer,
            split_decision,
        })
    }

    pub fn bridge<P, C>(
        &mut self,
        producer: P,
        consumer: C,
        split_decision: &IndexedSplitDecision,
    ) -> C::Output
    where
        P: IndexedProducer,
        C: Consumer<IntoCollector: Collector<P::Item>>,
    {
        struct IndexedJob<'a, P, C> {
            producer: P,
            consumer: C,
            split_decision: &'a IndexedSplitDecision,
        }

        impl<P, C> Job for IndexedJob<'_, P, C>
        where
            P: IndexedProducer,
            C: Consumer<IntoCollector: Collector<P::Item>>,
        {
            type Output = C::Output;

            fn execute<'a>(self, state: RcrcSharedState<'a>) -> impl Future<Output = Self::Output>
            where
                Self: 'a,
            {
                bridge_task_indexed(state, self.producer, self.consumer, self.split_decision)
            }
        }

        self.spawn(IndexedJob {
            producer,
            consumer,
            split_decision,
        })
    }

    fn spawn<J: Job>(&mut self, job: J) -> J::Output {
        let state = Rc::new(RefCell::new(SharedState {
            queue: vec![],
            rng: self.rng.clone(),
        }));

        let channel = Rc::new(RefCell::new(None));

        let state_clone = Rc::clone(&state);
        let channel_clone = Rc::clone(&channel);
        state.borrow_mut().queue.push(Box::pin(async move {
            let state = state_clone;
            let channel = channel_clone;
            let output = job.execute(state).await;
            let _ = channel.borrow_mut().insert(output);
        }));

        loop {
            let mut state_ref = state.borrow_mut();
            if state_ref.queue.is_empty() {
                break;
            }

            let idx = self.rng.random_range(..state_ref.queue.len());
            let mut task = state_ref.queue.swap_remove(idx);
            drop(state_ref);

            let mut cx = Context::from_waker(Waker::noop());
            if task.as_mut().poll(&mut cx).is_pending() {
                state.borrow_mut().queue.push(task);
            }
        }

        // Update the state after running all the jobs.
        self.rng.clone_from(&state.borrow().rng);

        channel
            .borrow_mut()
            .take()
            .expect("the task isn't executed successfully")
    }
}

impl<'a> SharedState<'a> {
    fn spawn<F>(&mut self, task: F) -> impl Future<Output = F::Output> + use<F>
    where
        F: Future + 'a,
    {
        let channel = Rc::new(RefCell::new(None));
        let channel_clone = Rc::clone(&channel);

        self.queue.push(Box::pin(async move {
            let channel = channel_clone;
            let output = task.await;
            let _ = channel.borrow_mut().insert(output);
        }));

        std::future::poll_fn(move |cx| {
            if let Some(value) = channel.borrow_mut().take() {
                Poll::Ready(value)
            } else {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
    }
}

async fn bridge_task<'a, 'sd: 'a, P, C>(
    state: Rc<RefCell<SharedState<'a>>>,
    mut producer: P,
    consumer: C,
    split_decision: &'sd UnindexedSplitDecision,
) -> C::Output
where
    P: Producer + 'a,
    C: UnindexedConsumer<IntoCollector: Collector<P::Item>> + 'a,
{
    yield_now().await;

    match split_decision {
        UnindexedSplitDecision::Stay => {
            let mut iter = producer.into_iter();
            let mut collector = consumer.into_collector();
            yield_now().await;

            if collector.break_hint().is_break() {
                yield_now().await;
                return collector.finish();
            }

            loop {
                // Dp this cuz of the stupid `clippy::await_holding_refcell_ref` lint
                // not understanding that we don't actually hold any `RefMut`
                // across an `.await` point.
                let method = {
                    let mut state = state.borrow_mut();
                    state.rng.sample(CollectDistribution)
                };

                match method {
                    CollectMethod::Collect => {
                        let Some(item) = iter.next() else {
                            break delay_output(collector.finish()).await;
                        };

                        if collector.collect(item).is_break() {
                            break delay_output(collector.finish()).await;
                        }
                    }
                    CollectMethod::CollectThenFinish => {
                        break delay_output(collector.collect_then_finish(iter)).await;
                    }
                    CollectMethod::CollectMany { n } => {
                        if collector.collect_many(iter.by_ref().take(n)).is_break() {
                            break delay_output(collector.finish()).await;
                        }
                    }
                }

                yield_now().await;
            }
        }
        UnindexedSplitDecision::Split { left, right } => {
            let producer_left = producer.split_off_left();
            let consumer_left = consumer.split_off_left();
            let combiner = consumer.to_combiner();
            let producer_right = producer;
            let consumer_right = consumer;
            let state_left = Rc::clone(&state);
            let state_right = Rc::clone(&state);
            yield_now().await;

            let (left_work, right_work) = {
                let mut state = state.borrow_mut();
                let left_work = state.spawn(bridge_task(state_left, producer_left, consumer_left, left));
                let right_work = state.spawn(bridge_task(state_right, producer_right, consumer_right, right));

                (left_work, right_work)
            };
            yield_now().await;

            let left_output = left_work.await;
            yield_now().await;

            let right_output = right_work.await;
            yield_now().await;

            let mut output = left_output;
            combiner.combine(&mut output, right_output);
            yield_now().await;

            output
        }
    }
}

async fn bridge_task_indexed<'a, 'sd: 'a, P, C>(
    state: Rc<RefCell<SharedState<'a>>>,
    mut producer: P,
    mut consumer: C,
    split_decision: &'sd IndexedSplitDecision,
) -> C::Output
where
    P: IndexedProducer + 'a,
    C: Consumer<IntoCollector: Collector<P::Item>> + 'a,
{
    yield_now().await;

    match split_decision {
        IndexedSplitDecision::Stay => {
            let mut iter = producer.into_iter();
            let mut collector = consumer.into_collector();
            yield_now().await;

            if collector.break_hint().is_break() {
                yield_now().await;
                return collector.finish();
            }

            loop {
                // Dp this cuz of the stupid `clippy::await_holding_refcell_ref` lint
                // not understanding that we don't actually hold any `RefMut`
                // across an `.await` point.
                let method = {
                    let mut state = state.borrow_mut();
                    state.rng.sample(CollectDistribution)
                };

                match method {
                    CollectMethod::Collect => {
                        let Some(item) = iter.next() else {
                            break delay_output(collector.finish()).await;
                        };

                        if collector.collect(item).is_break() {
                            break delay_output(collector.finish()).await;
                        }
                    }
                    CollectMethod::CollectThenFinish => {
                        break delay_output(collector.collect_then_finish(iter)).await;
                    }
                    CollectMethod::CollectMany { n } => {
                        if collector.collect_many(iter.by_ref().take(n)).is_break() {
                            break delay_output(collector.finish()).await;
                        }
                    }
                }

                yield_now().await;
            }
        }
        IndexedSplitDecision::Split { left, right, at } => {
            let at = *at;
            let (producer_left, producer_right) = (producer.split_off_left_at(at), producer);
            let ((consumer_left, combiner), consumer_right) = (consumer.split_off_left_at(at), consumer);
            let state_left = Rc::clone(&state);
            let state_right = Rc::clone(&state);
            yield_now().await;

            let (left_work, right_work) = {
                let mut state = state.borrow_mut();
                let left_work = state.spawn(bridge_task_indexed(
                    state_left,
                    producer_left,
                    consumer_left,
                    left,
                ));
                let right_work = state.spawn(bridge_task_indexed(
                    state_right,
                    producer_right,
                    consumer_right,
                    right,
                ));

                (left_work, right_work)
            };
            yield_now().await;

            let left_output = left_work.await;
            yield_now().await;

            let right_output = right_work.await;
            yield_now().await;

            let mut output = left_output;
            combiner.combine(&mut output, right_output);
            yield_now().await;

            output
        }
    }
}

async fn yield_now() {
    let mut ready = false;
    std::future::poll_fn(move |cx| {
        if ready {
            Poll::Ready(())
        } else {
            ready = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await
}

async fn delay_output<O>(output: O) -> O {
    yield_now().await;
    output
}

trait Job {
    type Output;

    fn execute<'a>(self, state: RcrcSharedState<'a>) -> impl Future<Output = Self::Output>
    where
        Self: 'a;
}

enum CollectMethod {
    /// Use [`Collector::collect()`] method.
    Collect,
    /// Use [`Collector::collect_then_finish()`] method.
    CollectThenFinish,
    // FIXME: if we ever have to collect more than 10 items (very unlikely),
    // we restructure.
    /// Use [`Collector::collect_many()`] method for the maximum of `n` items.
    CollectMany { n: usize },
}

struct CollectDistribution;

impl Distribution<CollectMethod> for CollectDistribution {
    fn sample<R: rand::prelude::Rng + ?Sized>(&self, rng: &mut R) -> CollectMethod {
        match rng.random_range(0..3) {
            0 => CollectMethod::Collect,
            1 => CollectMethod::CollectThenFinish,
            _ => CollectMethod::CollectMany {
                n: rng.random_range(0..=10),
            },
        }
    }
}
