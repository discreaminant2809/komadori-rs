use std::{
    cell::RefCell,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use komadori::prelude::*;
use rand::{RngExt, SeedableRng, rngs::StdRng};

use crate::collector::plumbing::{Combiner, UnindexedConsumer};

use super::{Producer, UnindexedSplitDecision};

pub struct CoroutinePool {
    rng: StdRng,
}

pub enum Event {
    StartBridging,
    StayCreateSerialCollector,
    StayUsingSerialCollector,
    StayReturn,
}

type Work<'a> = Pin<Box<dyn Future<Output = ()> + 'a>>;
type Queue<'a> = Vec<Work<'a>>;

#[derive(Default)]
struct SharedState<'a> {
    queue: Queue<'a>,
    task_pick_log: Vec<usize>,
    event_log: Vec<Event>,
}

impl CoroutinePool {
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn bridge<P, C>(
        &mut self,
        producer: P,
        consumer: C,
        split_decision: UnindexedSplitDecision,
    ) -> C::Output
    where
        P: Producer,
        C: UnindexedConsumer<IntoCollector: Collector<P::Item>>,
    {
        let state = Rc::new(RefCell::new(SharedState::default()));

        let channel = Rc::new(RefCell::new(None));

        let state_clone = Rc::clone(&state);
        let channel_clone = Rc::clone(&channel);
        state.borrow_mut().queue.push(Box::pin(async move {
            let state = state_clone;
            let channel = channel_clone;
            let output = bridge_task(state, producer, consumer, split_decision).await;
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

        channel.borrow_mut().take().expect("")
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

async fn bridge_task<'a, P, C>(
    state: Rc<RefCell<SharedState<'a>>>,
    mut producer: P,
    consumer: C,
    split_decision: UnindexedSplitDecision,
) -> C::Output
where
    P: Producer + 'a,
    C: UnindexedConsumer<IntoCollector: Collector<P::Item>> + 'a,
{
    yield_now().await;

    match split_decision {
        UnindexedSplitDecision::Stay => {
            let iter = producer.into_iter();
            let collector = consumer.into_collector();
            yield_now().await;

            let output = iter.feed_into(collector);
            yield_now().await;

            output
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
                let left_work = state.spawn(bridge_task(state_left, producer_left, consumer_left, *left));
                let right_work =
                    state.spawn(bridge_task(state_right, producer_right, consumer_right, *right));

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

fn _asjkndsanjk() {
    // let mut pool = CoroutinePool::new();

    // pool.bridge();
}
