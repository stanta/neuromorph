//! Bounded, single-writer CPU shards for independently addressable DANMA neurons.
//!
//! A worker owns many neuron states and processes their forward/backward
//! messages sequentially. The number of OS threads and queued commands is
//! bounded independently of the number of neurons. This prototype keeps the
//! existing danma-core BTreeMap representation; dense SoA/CSR storage is a
//! separate, measurable optimization.

use danma_core::{
    ActivationTrace, Error as CoreError, Feedback, FeedbackStatus, Forward, Neuron,
    NeuronId, EventId,
};
use std::{
    collections::BTreeMap,
    thread,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{mpsc, oneshot};

const MAX_CPU_WORKERS: usize = 64;
const MAX_MAILBOX_CAPACITY: usize = 4_096;
const MAX_NEURONS_PER_NODE: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardError {
    InvalidConfiguration,
    DuplicateNeuron(NeuronId),
    UnknownNeuron(NeuronId),
    Busy,
    WorkerStopped,
    Core(CoreError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NeuronInfo {
    pub id: NeuronId,
    pub version: u64,
    pub bias: f32,
    pub weights: BTreeMap<NeuronId, f32>,
}

enum BackwardClock {
    /// Deterministic clock supplied by reference-model tests.
    Fixed(u64),
    /// The worker checks the monotonic deadline immediately before mutation.
    LiveUntil(Instant),
}

fn epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

enum Command {
    Forward {
        target: NeuronId,
        input: Forward,
        answer: oneshot::Sender<Result<f32, ShardError>>,
    },
    Backward {
        target: NeuronId,
        input: Feedback,
        clock: BackwardClock,
        answer: oneshot::Sender<Result<FeedbackStatus, ShardError>>,
    },
    Inspect {
        target: NeuronId,
        answer: oneshot::Sender<Result<NeuronInfo, ShardError>>,
    },
    Trace {
        target: NeuronId,
        event_id: EventId,
        answer: oneshot::Sender<Result<Option<ActivationTrace>, ShardError>>,
    },
}

fn process(command: Command, neurons: &mut BTreeMap<NeuronId, Neuron>) {
    match command {
        Command::Forward { target, input, answer } => {
            let response = neurons
                .get_mut(&target)
                .ok_or(ShardError::UnknownNeuron(target))
                .and_then(|neuron| neuron.forward(input).map_err(ShardError::Core));
            let _ = answer.send(response);
        }
        Command::Backward { target, input, clock, answer } => {
            let now_ms = match clock {
                BackwardClock::Fixed(now_ms) => now_ms,
                BackwardClock::LiveUntil(deadline) => {
                    if Instant::now() >= deadline {
                        let _ = answer.send(Ok(FeedbackStatus::Expired));
                        return;
                    }
                    epoch_millis()
                }
            };
            let response = neurons
                .get_mut(&target)
                .ok_or(ShardError::UnknownNeuron(target))
                .and_then(|neuron| neuron.backward(input, now_ms).map_err(ShardError::Core));
            let _ = answer.send(response);
        }
        Command::Inspect { target, answer } => {
            let response = neurons
                .get(&target)
                .ok_or(ShardError::UnknownNeuron(target))
                .map(|neuron| NeuronInfo {
                    id: neuron.id(),
                    version: neuron.version(),
                    bias: neuron.bias(),
                    weights: neuron.weights().clone(),
                });
            let _ = answer.send(response);
        }
        Command::Trace { target, event_id, answer } => {
            let response = neurons
                .get(&target)
                .ok_or(ShardError::UnknownNeuron(target))
                .map(|neuron| neuron.trace(event_id));
            let _ = answer.send(response);
        }
    }
}

/// Immutable owner map + bounded per-worker mailboxes. Each neuron has
/// exactly one CPU worker owner. No CPU work runs on the Tokio I/O reactor.
///
/// This in-memory ownership is not a durable checkpoint or a fencing lease.
/// Worker threads terminate when all senders are dropped and mailboxes drain.
pub struct Shard {
    owners: BTreeMap<NeuronId, usize>,
    workers: Vec<mpsc::Sender<Command>>,
}

impl Shard {
    /// worker_limit is an upper bound, not one thread per neuron.
    /// mailbox_capacity limits pending commands per worker.
    pub fn new(
        neurons: Vec<Neuron>,
        worker_limit: usize,
        mailbox_capacity: usize,
    ) -> Result<Self, ShardError> {
        if neurons.is_empty()
            || neurons.len() > MAX_NEURONS_PER_NODE
            || worker_limit == 0
            || worker_limit > MAX_CPU_WORKERS
            || mailbox_capacity == 0
            || mailbox_capacity > MAX_MAILBOX_CAPACITY
        {
            return Err(ShardError::InvalidConfiguration);
        }

        let worker_count = worker_limit.min(neurons.len());
        let mut owners = BTreeMap::new();
        let mut partitions: Vec<BTreeMap<NeuronId, Neuron>> =
            (0..worker_count).map(|_| BTreeMap::new()).collect();

        for neuron in neurons {
            let id = neuron.id();
            let worker = (id % worker_count as u64) as usize;
            if owners.insert(id, worker).is_some() {
                return Err(ShardError::DuplicateNeuron(id));
            }
            partitions[worker].insert(id, neuron);
        }

        let mut workers = Vec::with_capacity(worker_count);
        for (index, mut partition) in partitions.into_iter().enumerate() {
            let (sender, mut receiver) = mpsc::channel(mailbox_capacity);
            thread::Builder::new()
                .name(format!("danma-cpu-{index}"))
                .spawn(move || {
                    while let Some(command) = receiver.blocking_recv() {
                        process(command, &mut partition);
                    }
                })
                .map_err(|_| ShardError::WorkerStopped)?;
            workers.push(sender);
        }

        Ok(Self { owners, workers })
    }

    pub fn neuron_ids(&self) -> Vec<NeuronId> {
        self.owners.keys().copied().collect()
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    pub fn contains(&self, id: NeuronId) -> bool {
        self.owners.contains_key(&id)
    }

    async fn execute<T>(
        &self,
        target: NeuronId,
        command: Command,
        answer: oneshot::Receiver<Result<T, ShardError>>,
    ) -> Result<T, ShardError> {
        let worker = *self
            .owners
            .get(&target)
            .ok_or(ShardError::UnknownNeuron(target))?;
        self.workers[worker]
            .try_send(command)
            .map_err(|err| match err {
                mpsc::error::TrySendError::Full(_) => ShardError::Busy,
                mpsc::error::TrySendError::Closed(_) => ShardError::WorkerStopped,
            })?;
        answer.await.map_err(|_| ShardError::WorkerStopped)?
    }

    pub async fn forward(&self, target: NeuronId, input: Forward) -> Result<f32, ShardError> {
        let (sender, receiver) = oneshot::channel();
        self.execute(target, Command::Forward { target, input, answer: sender }, receiver)
            .await
    }

    pub async fn backward(
        &self,
        target: NeuronId,
        input: Feedback,
        now_ms: u64,
    ) -> Result<FeedbackStatus, ShardError> {
        let (sender, receiver) = oneshot::channel();
        self.execute(
            target,
            Command::Backward {
                target,
                input,
                clock: BackwardClock::Fixed(now_ms),
                answer: sender,
            },
            receiver,
        ).await
    }

    /// Network-facing entrypoint. A queued message that outlives the
    /// monotonic transport deadline is rejected before any weight mutation.
    pub async fn backward_live(
        &self,
        target: NeuronId,
        input: Feedback,
        deadline: Instant,
    ) -> Result<FeedbackStatus, ShardError> {
        let (sender, receiver) = oneshot::channel();
        self.execute(
            target,
            Command::Backward {
                target,
                input,
                clock: BackwardClock::LiveUntil(deadline),
                answer: sender,
            },
            receiver,
        ).await
    }

    pub async fn inspect(&self, target: NeuronId) -> Result<NeuronInfo, ShardError> {
        let (sender, receiver) = oneshot::channel();
        self.execute(target, Command::Inspect { target, answer: sender }, receiver)
            .await
    }

    pub async fn trace(
        &self,
        target: NeuronId,
        event_id: EventId,
    ) -> Result<Option<ActivationTrace>, ShardError> {
        let (sender, receiver) = oneshot::channel();
        self.execute(
            target,
            Command::Trace { target, event_id, answer: sender },
            receiver,
        ).await
    }
}
