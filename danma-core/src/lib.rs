//! CPU-first DANMA neuron core.
//!
//! An EventID identifies one forward activation, not the entire thought.
//! Feedback is accumulated by downstream branch, then applied once locally.
//! No network-wide clock or replicated execution is required by this core.
//!
//! Time values are supplied by the caller. The first prototype assumes a
//! comparable millisecond clock across peers; a later wire protocol should
//! use relative TTL plus local monotonic deadlines to handle clock skew.

use std::collections::{BTreeMap, BTreeSet};

pub type NeuronId = u64;
pub type EdgeId = u64;
pub type EventId = u128;
pub type TraceId = u128;

/// Deterministic v1 activation identity for autonomous forward propagation.
///
/// The wire protocol currently carries u64 EventIDs, so the mixed result is
/// deliberately kept in that range. One neuron fires at most once per TraceID;
/// recurrent time-step semantics require a future explicit StepID.
pub fn derived_event_id(trace_id: TraceId, neuron_id: NeuronId) -> EventId {
    let folded = (trace_id as u64) ^ ((trace_id >> 64) as u64);
    let mut value = folded
        ^ neuron_id.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ 0xD1B5_4A32_D192_ED03;
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    EventId::from(if value == 0 { 1 } else { value })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidConfiguration,
    DuplicateWeight,
    DuplicateAxon,
    DuplicateAxonTarget,
    UnknownInput,
    DuplicateInput,
    ConflictingInput,
    DuplicateExpected,
    DuplicateEvent,
    CapacityExceeded,
    NonFinite,
    NumericOverflow,
    UnknownEvent,
    UnexpectedContributor,
    HopLimitExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    Linear,
    Relu,
}

impl Activation {
    fn forward(self, x: f32) -> f32 {
        match self {
            Self::Linear => x,
            Self::Relu => x.max(0.0),
        }
    }

    fn derivative(self, x: f32) -> f32 {
        match self {
            Self::Linear => 1.0,
            Self::Relu => {
                if x > 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub activation: Activation,
    pub learning_rate: f32,
    pub activation_ttl_ms: u64,
    pub replay_retention_ms: u64,
    /// Upper bound shared by active traces and completed-event tombstones.
    pub max_live_events: usize,
    /// Maximum weight updates since the activation's forward pass.
    pub max_staleness_versions: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SynapticInput {
    pub from: NeuronId,
    pub source_event_id: EventId,
    pub value: f32,
}

/// A logical outgoing synapse. Physical node addresses remain in the runtime
/// route table so neuron migration never requires rewriting axons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Axon {
    pub edge_id: EdgeId,
    pub to: NeuronId,
}

/// One independently delivered forward contribution to a downstream neuron.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ForwardSignal {
    pub event_id: EventId,
    pub trace_id: TraceId,
    pub now_ms: u64,
    pub input: SynapticInput,
    pub training: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SignalStatus {
    Pending { remaining: usize },
    Fired { output: f32 },
    IgnoredDuplicate,
    Expired,
}

/// The expected branch identities must be known when a forward activation
/// is committed. The caller assigns downstream EventIDs before dispatch.
#[derive(Debug, Clone)]
pub struct Forward {
    pub event_id: EventId,
    pub trace_id: TraceId,
    pub now_ms: u64,
    pub inputs: Vec<SynapticInput>,
    /// Empty means inference: no activation trace is kept for training.
    pub expected: Vec<FeedbackSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeedbackSource {
    Teacher,
    Neuron {
        neuron_id: NeuronId,
        event_id: EventId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Feedback {
    pub event_id: EventId,
    pub from: FeedbackSource,
    /// dLoss / dOutput for this particular downstream branch.
    pub gradient: f32,
    /// Prototype absolute deadline in a comparable millisecond clock.
    pub expires_at_ms: u64,
    /// Remaining number of neuron hops, including the current receiver.
    pub hops_left: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeedbackDispatch {
    pub target_neuron_id: NeuronId,
    pub feedback: Feedback,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeedbackStatus {
    Pending { remaining: usize },
    Applied {
        upstream: Vec<FeedbackDispatch>,
        version: u64,
    },
    IgnoredDuplicate,
    Expired,
    Stale,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActivationTrace {
    pub trace_id: TraceId,
    pub output: f32,
    pub parameter_version: u64,
    pub expires_at_ms: u64,
    pub expected_contributions: usize,
    pub received_contributions: usize,
}

#[derive(Debug, Clone, Copy)]
struct InputSnapshot {
    from: NeuronId,
    source_event_id: EventId,
    value: f32,
    weight_at_forward: f32,
}

#[derive(Debug, Clone, Copy)]
struct FeedbackPart {
    gradient: f32,
    expires_at_ms: u64,
    hops_left: u8,
}

#[derive(Debug)]
struct ForwardCollection {
    trace_id: TraceId,
    expires_at_ms: u64,
    training: bool,
    inputs: BTreeMap<NeuronId, SynapticInput>,
}

#[derive(Debug)]
struct ActivationRecord {
    trace_id: TraceId,
    preactivation: f32,
    output: f32,
    parameter_version: u64,
    expires_at_ms: u64,
    inputs: Vec<InputSnapshot>,
    expected: BTreeSet<FeedbackSource>,
    received: BTreeMap<FeedbackSource, FeedbackPart>,
}

#[derive(Debug, Clone, Copy)]
enum ClosedKind {
    Completed,
    Expired,
    Inference,
    Stale,
}

#[derive(Debug, Clone, Copy)]
struct ClosedEvent {
    until_ms: u64,
    kind: ClosedKind,
}

/// A single-writer actor: serialize calls through its owning shard/mailbox.
/// Crash-safe exactly-once effects require a durable journal, not included here.
#[derive(Debug)]
pub struct Neuron {
    id: NeuronId,
    bias: f32,
    weights: BTreeMap<NeuronId, f32>,
    axons: Vec<Axon>,
    version: u64,
    config: Config,
    collecting: BTreeMap<EventId, ForwardCollection>,
    pending: BTreeMap<EventId, ActivationRecord>,
    closed: BTreeMap<EventId, ClosedEvent>,
}

impl Neuron {
    pub fn new(
        id: NeuronId,
        bias: f32,
        weights: impl IntoIterator<Item = (NeuronId, f32)>,
        config: Config,
    ) -> Result<Self, Error> {
        Self::new_with_axons(id, bias, weights, std::iter::empty(), config)
    }

    pub fn new_with_axons(
        id: NeuronId,
        bias: f32,
        weights: impl IntoIterator<Item = (NeuronId, f32)>,
        axons: impl IntoIterator<Item = Axon>,
        config: Config,
    ) -> Result<Self, Error> {
        if id == 0
            || !bias.is_finite()
            || !config.learning_rate.is_finite()
            || config.learning_rate <= 0.0
            || config.activation_ttl_ms == 0
            || config.replay_retention_ms == 0
            || config.max_live_events == 0
        {
            return Err(Error::InvalidConfiguration);
        }

        let mut stored_weights = BTreeMap::new();
        for (source, weight) in weights {
            if source == 0 || !weight.is_finite() {
                return Err(Error::InvalidConfiguration);
            }
            if stored_weights.insert(source, weight).is_some() {
                return Err(Error::DuplicateWeight);
            }
        }

        let mut stored_axons = Vec::new();
        let mut edge_ids = BTreeSet::new();
        let mut targets = BTreeSet::new();
        for axon in axons {
            if axon.edge_id == 0 || axon.to == 0 {
                return Err(Error::InvalidConfiguration);
            }
            if !edge_ids.insert(axon.edge_id) {
                return Err(Error::DuplicateAxon);
            }
            if !targets.insert(axon.to) {
                return Err(Error::DuplicateAxonTarget);
            }
            stored_axons.push(axon);
        }
        stored_axons.sort_unstable();

        Ok(Self {
            id,
            bias,
            weights: stored_weights,
            axons: stored_axons,
            version: 0,
            config,
            collecting: BTreeMap::new(),
            pending: BTreeMap::new(),
            closed: BTreeMap::new(),
        })
    }

    pub fn id(&self) -> NeuronId {
        self.id
    }

    pub fn bias(&self) -> f32 {
        self.bias
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn weight(&self, source: NeuronId) -> Option<f32> {
        self.weights.get(&source).copied()
    }

    pub fn weights(&self) -> &BTreeMap<NeuronId, f32> {
        &self.weights
    }

    pub fn axons(&self) -> &[Axon] {
        &self.axons
    }

    pub fn trace(&self, event_id: EventId) -> Option<ActivationTrace> {
        self.pending.get(&event_id).map(|record| ActivationTrace {
            trace_id: record.trace_id,
            output: record.output,
            parameter_version: record.parameter_version,
            expires_at_ms: record.expires_at_ms,
            expected_contributions: record.expected.len(),
            received_contributions: record.received.len(),
        })
    }

    pub fn forward(&mut self, request: Forward) -> Result<f32, Error> {
        self.expire(request.now_ms);
        if request.event_id == 0 {
            return Err(Error::InvalidConfiguration);
        }
        if self.collecting.contains_key(&request.event_id)
            || self.pending.contains_key(&request.event_id)
            || self.closed.contains_key(&request.event_id)
        {
            return Err(Error::DuplicateEvent);
        }
        if self.collecting.len() + self.pending.len() + self.closed.len()
            >= self.config.max_live_events
        {
            return Err(Error::CapacityExceeded);
        }

        let expires_at_ms = request
            .now_ms
            .checked_add(self.config.activation_ttl_ms)
            .ok_or(Error::InvalidConfiguration)?;

        let expected: BTreeSet<_> = request.expected.iter().copied().collect();
        if expected.len() != request.expected.len() {
            return Err(Error::DuplicateExpected);
        }

        let mut seen_inputs = BTreeSet::new();
        let mut inputs = Vec::with_capacity(request.inputs.len());
        let mut z = self.bias;
        for input in &request.inputs {
            if !input.value.is_finite() {
                return Err(Error::NonFinite);
            }
            if !seen_inputs.insert(input.from) {
                return Err(Error::DuplicateInput);
            }
            let weight = *self.weights.get(&input.from).ok_or(Error::UnknownInput)?;
            z += weight * input.value;
            if !z.is_finite() {
                return Err(Error::NumericOverflow);
            }
            inputs.push(InputSnapshot {
                from: input.from,
                source_event_id: input.source_event_id,
                value: input.value,
                weight_at_forward: weight,
            });
        }

        let output = self.config.activation.forward(z);
        if expected.is_empty() {
            self.closed.insert(
                request.event_id,
                ClosedEvent {
                    until_ms: expires_at_ms.saturating_add(self.config.replay_retention_ms),
                    kind: ClosedKind::Inference,
                },
            );
        } else {
            self.pending.insert(
                request.event_id,
                ActivationRecord {
                    trace_id: request.trace_id,
                    preactivation: z,
                    output,
                    parameter_version: self.version,
                    expires_at_ms,
                    inputs,
                    expected,
                    received: BTreeMap::new(),
                },
            );
        }
        Ok(output)
    }

    /// Accept one forward contribution. Fan-in uses a barrier over the
    /// configured dendrite sources; duplicate transport delivery is harmless.
    pub fn receive_signal(
        &mut self,
        signal: ForwardSignal,
    ) -> Result<SignalStatus, Error> {
        self.expire(signal.now_ms);
        if signal.event_id == 0 || !signal.input.value.is_finite() {
            return Err(if signal.event_id == 0 {
                Error::InvalidConfiguration
            } else {
                Error::NonFinite
            });
        }
        if !self.weights.contains_key(&signal.input.from) {
            return Err(Error::UnknownInput);
        }
        if let Some(closed) = self.closed.get(&signal.event_id) {
            return Ok(match closed.kind {
                ClosedKind::Expired => SignalStatus::Expired,
                ClosedKind::Completed | ClosedKind::Inference | ClosedKind::Stale => {
                    SignalStatus::IgnoredDuplicate
                }
            });
        }
        if self.pending.contains_key(&signal.event_id) {
            return Ok(SignalStatus::IgnoredDuplicate);
        }

        if let Some(collection) = self.collecting.get(&signal.event_id) {
            if collection.trace_id != signal.trace_id || collection.training != signal.training {
                return Err(Error::DuplicateEvent);
            }
            if let Some(previous) = collection.inputs.get(&signal.input.from) {
                return if *previous == signal.input {
                    Ok(SignalStatus::IgnoredDuplicate)
                } else {
                    Err(Error::ConflictingInput)
                };
            }
        } else {
            if self.collecting.len() + self.pending.len() + self.closed.len()
                >= self.config.max_live_events
            {
                return Err(Error::CapacityExceeded);
            }
            let expires_at_ms = signal
                .now_ms
                .checked_add(self.config.activation_ttl_ms)
                .ok_or(Error::InvalidConfiguration)?;
            self.collecting.insert(
                signal.event_id,
                ForwardCollection {
                    trace_id: signal.trace_id,
                    expires_at_ms,
                    training: signal.training,
                    inputs: BTreeMap::new(),
                },
            );
        }

        let collection = self
            .collecting
            .get_mut(&signal.event_id)
            .ok_or(Error::UnknownEvent)?;
        collection.inputs.insert(signal.input.from, signal.input);
        if collection.inputs.len() < self.weights.len() {
            return Ok(SignalStatus::Pending {
                remaining: self.weights.len() - collection.inputs.len(),
            });
        }

        let collection = self
            .collecting
            .remove(&signal.event_id)
            .ok_or(Error::UnknownEvent)?;
        let expected = if collection.training {
            if self.axons.is_empty() {
                vec![FeedbackSource::Teacher]
            } else {
                self.axons
                    .iter()
                    .map(|axon| FeedbackSource::Neuron {
                        neuron_id: axon.to,
                        event_id: derived_event_id(collection.trace_id, axon.to),
                    })
                    .collect()
            }
        } else {
            Vec::new()
        };
        let output = self.forward(Forward {
            event_id: signal.event_id,
            trace_id: collection.trace_id,
            now_ms: signal.now_ms,
            inputs: collection.inputs.into_values().collect(),
            expected,
        })?;
        Ok(SignalStatus::Fired { output })
    }

    /// Expire partial fan-in, active traces and completed-event tombstones.
    /// A removed event cannot be learned without an outstanding activation.
    pub fn expire(&mut self, now_ms: u64) {
        self.closed.retain(|_, closed| closed.until_ms > now_ms);

        let expired_collecting: Vec<_> = self
            .collecting
            .iter()
            .filter(|(_, record)| record.expires_at_ms <= now_ms)
            .map(|(event_id, record)| (*event_id, record.expires_at_ms))
            .collect();
        for (event_id, expires_at_ms) in expired_collecting {
            self.collecting.remove(&event_id);
            self.closed.insert(
                event_id,
                ClosedEvent {
                    until_ms: expires_at_ms.saturating_add(self.config.replay_retention_ms),
                    kind: ClosedKind::Expired,
                },
            );
        }

        let expired: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, record)| record.expires_at_ms <= now_ms)
            .map(|(event_id, _)| *event_id)
            .collect();
        for event_id in expired {
            self.close(event_id, ClosedKind::Expired);
        }
    }

    fn close(&mut self, event_id: EventId, kind: ClosedKind) {
        if let Some(record) = self.pending.remove(&event_id) {
            self.closed.insert(
                event_id,
                ClosedEvent {
                    until_ms: record
                        .expires_at_ms
                        .saturating_add(self.config.replay_retention_ms),
                    kind,
                },
            );
        }
    }

    /// Aggregate one contribution per expected branch and update weights
    /// exactly once for the activation, subject to its local retention window.
    pub fn backward(
        &mut self,
        packet: Feedback,
        now_ms: u64,
    ) -> Result<FeedbackStatus, Error> {
        self.expire(now_ms);
        if let Some(closed) = self.closed.get(&packet.event_id) {
            return Ok(match closed.kind {
                ClosedKind::Expired => FeedbackStatus::Expired,
                ClosedKind::Completed | ClosedKind::Inference | ClosedKind::Stale => {
                    FeedbackStatus::IgnoredDuplicate
                }
            });
        }
        let record = self.pending.get(&packet.event_id).ok_or(Error::UnknownEvent)?;
        if packet.expires_at_ms <= now_ms {
            return Ok(FeedbackStatus::Expired);
        }
        if packet.hops_left == 0 {
            return Err(Error::HopLimitExhausted);
        }
        if !packet.gradient.is_finite() {
            return Err(Error::NonFinite);
        }
        if !record.expected.contains(&packet.from) {
            return Err(Error::UnexpectedContributor);
        }
        if record.received.contains_key(&packet.from) {
            return Ok(FeedbackStatus::IgnoredDuplicate);
        }
        let part = FeedbackPart {
            gradient: packet.gradient,
            expires_at_ms: packet.expires_at_ms,
            hops_left: packet.hops_left,
        };
        if record.received.len() + 1 < record.expected.len() {
            let remaining = record.expected.len() - record.received.len() - 1;
            self.pending
                .get_mut(&packet.event_id)
                .ok_or(Error::UnknownEvent)?
                .received
                .insert(packet.from, part);
            return Ok(FeedbackStatus::Pending { remaining });
        }

        // No mutation until the entire update is validated. This is the local
        // effect boundary; transport redelivery is harmless while tombstones live.
        let mut gradient = packet.gradient;
        let mut expires_at_ms = record.expires_at_ms.min(packet.expires_at_ms);
        let mut hops_left = packet.hops_left;
        for previous in record.received.values() {
            if previous.expires_at_ms <= now_ms {
                self.close(packet.event_id, ClosedKind::Expired);
                return Ok(FeedbackStatus::Expired);
            }
            gradient += previous.gradient;
            expires_at_ms = expires_at_ms.min(previous.expires_at_ms);
            hops_left = hops_left.min(previous.hops_left);
        }
        if !gradient.is_finite() {
            return Err(Error::NumericOverflow);
        }

        if self.version.saturating_sub(record.parameter_version)
            > self.config.max_staleness_versions
        {
            self.close(packet.event_id, ClosedKind::Stale);
            return Ok(FeedbackStatus::Stale);
        }

        let local_gradient =
            gradient * self.config.activation.derivative(record.preactivation);
        let new_bias = self.bias - self.config.learning_rate * local_gradient;
        if !local_gradient.is_finite() || !new_bias.is_finite() {
            return Err(Error::NumericOverflow);
        }
        let mut updated_weights = Vec::with_capacity(record.inputs.len());
        let mut upstream = Vec::with_capacity(record.inputs.len());
        for input in &record.inputs {
            let weight = *self.weights.get(&input.from).ok_or(Error::UnknownInput)?;
            let new_weight =
                weight - self.config.learning_rate * local_gradient * input.value;
            let upstream_gradient = local_gradient * input.weight_at_forward;
            if !new_weight.is_finite() || !upstream_gradient.is_finite() {
                return Err(Error::NumericOverflow);
            }
            updated_weights.push((input.from, new_weight));
            if hops_left > 1 {
                upstream.push(FeedbackDispatch {
                    target_neuron_id: input.from,
                    feedback: Feedback {
                        event_id: input.source_event_id,
                        from: FeedbackSource::Neuron {
                            neuron_id: self.id,
                            event_id: packet.event_id,
                        },
                        gradient: upstream_gradient,
                        expires_at_ms,
                        hops_left: hops_left - 1,
                    },
                });
            }
        }
        let next_version = self.version.checked_add(1).ok_or(Error::NumericOverflow)?;
        self.close(packet.event_id, ClosedKind::Completed);
        for (source, weight) in updated_weights {
            if let Some(stored) = self.weights.get_mut(&source) {
                *stored = weight;
            }
        }
        self.bias = new_bias;
        self.version = next_version;
        Ok(FeedbackStatus::Applied {
            upstream,
            version: next_version,
        })
    }
}
