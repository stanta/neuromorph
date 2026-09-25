//! Bounded TCP transport for the DANMA CPU-first prototype.
//!
//! This is a trusted *loopback-only* development cluster. Gossip shares route
//! advertisements; activation and feedback use direct, addressed TCP requests.
//! Neither gossip nor an acknowledgement provides durable exactly-once effects.
use danma_core::{
    Feedback, FeedbackSource, FeedbackStatus, Forward, Neuron, SynapticInput,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{RwLock, Semaphore},
    time::timeout,
};

const MAX_FRAME_BYTES: usize = 64 * 1024;
const MAX_ADVERTISED_ROUTES: usize = 128;
const MAX_IO_WAIT: Duration = Duration::from_secs(4);
const MAX_CONCURRENT_CONNECTIONS: usize = 32;
const MAX_INCOMING_INPUTS: usize = 128;
const MAX_EXPECTED_BRANCHES: usize = 128;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn elapsed_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

async fn read_frame(stream: &mut TcpStream) -> io::Result<Value> {
    let length = stream.read_u32().await? as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(invalid("frame exceeds protocol size limit"));
    }
    let mut bytes = vec![0_u8; length];
    stream.read_exact(&mut bytes).await?;
    serde_json::from_slice(&bytes).map_err(|_| invalid("invalid JSON frame"))
}

async fn write_frame(stream: &mut TcpStream, value: &Value) -> io::Result<()> {
    let encoded = serde_json::to_vec(value).map_err(|_| invalid("cannot serialize response"))?;
    if encoded.is_empty() || encoded.len() > MAX_FRAME_BYTES {
        return Err(invalid("outgoing frame exceeds size limit"));
    }
    stream.write_u32(encoded.len() as u32).await?;
    stream.write_all(&encoded).await?;
    stream.flush().await
}

/// One request per TCP connection. A timeout is an UNKNOWN delivery outcome,
/// not evidence that a remote learning update was rolled back.
pub async fn request(address: SocketAddr, value: &Value) -> io::Result<Value> {
    timeout(MAX_IO_WAIT, async {
        let mut stream = TcpStream::connect(address).await?;
        write_frame(&mut stream, value).await?;
        read_frame(&mut stream).await
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "DANMA request timed out"))?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Advert {
    pub owner: u64,
    pub neuron: u64,
    pub epoch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Source {
    Teacher,
    Neuron { neuron_id: u64, event_id: u64 },
}

impl From<Source> for FeedbackSource {
    fn from(source: Source) -> Self {
        match source {
            Source::Teacher => FeedbackSource::Teacher,
            Source::Neuron {
                neuron_id,
                event_id,
            } => FeedbackSource::Neuron {
                neuron_id,
                event_id: u128::from(event_id),
            },
        }
    }
}

impl TryFrom<FeedbackSource> for Source {
    type Error = io::Error;

    fn try_from(source: FeedbackSource) -> io::Result<Self> {
        Ok(match source {
            FeedbackSource::Teacher => Self::Teacher,
            FeedbackSource::Neuron {
                neuron_id,
                event_id,
            } => Self::Neuron {
                neuron_id,
                event_id: u64::try_from(event_id)
                    .map_err(|_| invalid("EventID exceeds v1 wire range"))?,
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    from: u64,
    source_event_id: u64,
    value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Message {
    Gossip {
        from_node: u64,
        routes: Vec<Advert>,
    },
    Routes,
    Inspect {
        target: u64,
        route_hops: u8,
    },
    Trace {
        target: u64,
        event_id: u64,
        route_hops: u8,
    },
    Forward {
        target: u64,
        event_id: u64,
        trace_id: u64,
        route_hops: u8,
        inputs: Vec<Input>,
        expected: Vec<Source>,
    },
    Backward {
        target: u64,
        event_id: u64,
        from: Source,
        gradient: f32,
        ttl_ms: u64,
        gradient_hops: u8,
        route_hops: u8,
    },
}

#[derive(Clone)]
pub struct Peer {
    pub id: u64,
    pub address: SocketAddr,
}

pub struct NodeConfig {
    pub id: u64,
    pub address: SocketAddr,
    pub neuron: Neuron,
    /// Explicitly permitted bootstrap peers. Only loopback is accepted by v1.
    pub peers: Vec<Peer>,
}

#[derive(Clone, Copy)]
struct Route {
    owner: u64,
    epoch: u64,
}

struct NodeState {
    id: u64,
    neuron_id: u64,
    neuron: Mutex<Neuron>,
    peers: BTreeMap<u64, SocketAddr>,
    routes: RwLock<BTreeMap<u64, Route>>,
}

impl NodeState {
    async fn target_address(&self, target: u64) -> Option<SocketAddr> {
        let route = *self.routes.read().await.get(&target)?;
        if route.owner == self.id {
            return None;
        }
        self.peers.get(&route.owner).copied()
    }

    async fn route_or_error(&self, target: u64, hops: u8) -> Result<SocketAddr, Value> {
        if hops < 2 {
            return Err(error_response("route_hops_exhausted"));
        }
        self.target_address(target)
            .await
            .ok_or_else(|| error_response("route_unknown"))
    }

    async fn process(&self, msg: Message) -> Value {
        match msg {
            Message::Routes => {
                let routes: BTreeMap<_, _> = self
                    .routes
                    .read()
                    .await
                    .iter()
                    .map(|(neuron, route)| (neuron.to_string(), route.owner))
                    .collect();
                json!({"kind":"routes_result","routes":routes})
            }
            Message::Gossip { from_node, routes } => {
                if !self.peers.contains_key(&from_node) || routes.len() > MAX_ADVERTISED_ROUTES {
                    return error_response("untrusted_or_oversized_gossip");
                }
                let mut table = self.routes.write().await;
                // Stage the full advertisement batch: bad routes must not
                // partially mutate the control-plane routing table.
                let mut staged = table.clone();
                for adv in routes {
                    if adv.owner == 0
                        || adv.neuron == 0
                        || adv.epoch == 0
                        || !(self.peers.contains_key(&adv.owner)
                            || (adv.owner == self.id && adv.neuron == self.neuron_id))
                    {
                        return error_response("invalid_route_owner");
                    }
                    if let Some(current) = staged.get(&adv.neuron) {
                        if current.owner != adv.owner {
                            return error_response("route_owner_conflict");
                        }
                        if current.epoch >= adv.epoch {
                            continue;
                        }
                    }
                    if staged.len() >= MAX_ADVERTISED_ROUTES && !staged.contains_key(&adv.neuron) {
                        return error_response("route_table_full");
                    }
                    staged.insert(
                        adv.neuron,
                        Route {
                            owner: adv.owner,
                            epoch: adv.epoch,
                        },
                    );
                }
                *table = staged;
                json!({"kind":"gossip_accepted"})
            }
            Message::Inspect { target, route_hops } => {
                if target != self.neuron_id {
                    let address = match self.route_or_error(target, route_hops).await {
                        Ok(address) => address,
                        Err(response) => return response,
                    };
                    return self
                        .relay(
                            address,
                            &Message::Inspect {
                                target,
                                route_hops: route_hops - 1,
                            },
                        )
                        .await;
                }
                let n = self.neuron.lock().expect("neuron mutex poisoned");
                json!({
                    "kind":"inspect_result",
                    "target":target,
                    "version":n.version(),
                    "bias":n.bias(),
                    "weights":n.weights()
                })
            }
            Message::Trace {
                target,
                event_id,
                route_hops,
            } => {
                if target != self.neuron_id {
                    let address = match self.route_or_error(target, route_hops).await {
                        Ok(address) => address,
                        Err(response) => return response,
                    };
                    return self
                        .relay(
                            address,
                            &Message::Trace {
                                target,
                                event_id,
                                route_hops: route_hops - 1,
                            },
                        )
                        .await;
                }
                let neuron = self.neuron.lock().expect("neuron mutex poisoned");
                let trace = neuron.trace(u128::from(event_id)).map(|trace| {
                    json!({
                        "trace_id":trace.trace_id,
                        "output":trace.output,
                        "parameter_version":trace.parameter_version,
                        "expires_at_ms":trace.expires_at_ms,
                        "expected_contributions":trace.expected_contributions,
                        "received_contributions":trace.received_contributions
                    })
                });
                json!({"kind":"trace_result","target":target,"event_id":event_id,"trace":trace})
            }
            Message::Forward {
                target,
                event_id,
                trace_id,
                route_hops,
                inputs,
                expected,
            } => {
                if inputs.len() > MAX_INCOMING_INPUTS || expected.len() > MAX_EXPECTED_BRANCHES {
                    return error_response("activation_too_large");
                }
                if target != self.neuron_id {
                    let address = match self.route_or_error(target, route_hops).await {
                        Ok(address) => address,
                        Err(response) => return response,
                    };
                    return self
                        .relay(
                            address,
                            &Message::Forward {
                                target,
                                event_id,
                                trace_id,
                                route_hops: route_hops - 1,
                                inputs,
                                expected,
                            },
                        )
                        .await;
                }
                let forward = Forward {
                    event_id: u128::from(event_id),
                    trace_id: u128::from(trace_id),
                    now_ms: elapsed_millis(),
                    inputs: inputs
                        .into_iter()
                        .map(|input| SynapticInput {
                            from: input.from,
                            source_event_id: u128::from(input.source_event_id),
                            value: input.value,
                        })
                        .collect(),
                    expected: expected.into_iter().map(FeedbackSource::from).collect(),
                };
                let mut n = self.neuron.lock().expect("neuron mutex poisoned");
                match n.forward(forward) {
                    Ok(output) => json!({"kind":"forward_result","output":output}),
                    Err(err) => error_response(&format!("forward_{err:?}")),
                }
            }
            Message::Backward {
                target,
                event_id,
                from,
                gradient,
                ttl_ms,
                gradient_hops,
                route_hops,
            } => {
                if ttl_ms == 0 {
                    return json!({"kind":"backward_result","status":"expired"});
                }
                let deadline = match Instant::now().checked_add(Duration::from_millis(ttl_ms)) {
                    Some(deadline) => deadline,
                    None => return error_response("invalid_ttl"),
                };
                if target != self.neuron_id {
                    let address = match self.route_or_error(target, route_hops).await {
                        Ok(address) => address,
                        Err(response) => return response,
                    };
                    let remaining = remaining_ms(deadline);
                    if remaining == 0 {
                        return json!({"kind":"backward_result","status":"expired"});
                    }
                    return self
                        .relay(
                            address,
                            &Message::Backward {
                                target,
                                event_id,
                                from,
                                gradient,
                                ttl_ms: remaining,
                                gradient_hops,
                                route_hops: route_hops - 1,
                            },
                        )
                        .await;
                }
                let now = elapsed_millis();
                let packet = Feedback {
                    event_id: u128::from(event_id),
                    from: from.into(),
                    gradient,
                    expires_at_ms: now.saturating_add(ttl_ms),
                    hops_left: gradient_hops,
                };
                let result = {
                    let mut n = self.neuron.lock().expect("neuron mutex poisoned");
                    n.backward(packet, now)
                };
                match result {
                    Err(err) => error_response(&format!("backward_{err:?}")),
                    Ok(FeedbackStatus::Pending { remaining }) => {
                        json!({"kind":"backward_result","status":"pending","remaining":remaining})
                    }
                    Ok(FeedbackStatus::IgnoredDuplicate) => {
                        json!({"kind":"backward_result","status":"ignored_duplicate"})
                    }
                    Ok(FeedbackStatus::Expired) => {
                        json!({"kind":"backward_result","status":"expired"})
                    }
                    Ok(FeedbackStatus::Stale) => {
                        json!({"kind":"backward_result","status":"stale"})
                    }
                    Ok(FeedbackStatus::Applied { upstream, version }) => {
                        let mut unrouted: Vec<Value> = Vec::new();
                        for dispatch in upstream {
                            let dest = dispatch.target_neuron_id;
                            let core_packet = dispatch.feedback;
                            let remaining = remaining_ms(deadline).min(
                                core_packet.expires_at_ms.saturating_sub(elapsed_millis()),
                            );
                            if remaining == 0 {
                                unrouted.push(json!({"target":dest,"reason":"expired"}));
                                continue;
                            }
                            let address = match self.target_address(dest).await {
                                Some(address) => address,
                                None => {
                                    unrouted.push(json!({
                                        "target":dest,
                                        "reason":"no_route",
                                        "event_id":core_packet.event_id.to_string(),
                                        "gradient":core_packet.gradient
                                    }));
                                    continue;
                                }
                            };
                            let next_source = match Source::try_from(core_packet.from) {
                                Ok(source) => source,
                                Err(_) => {
                                    unrouted.push(json!({"target":dest,"reason":"event_id_out_of_range"}));
                                    continue;
                                }
                            };
                            let parent_event = match u64::try_from(core_packet.event_id) {
                                Ok(id) => id,
                                Err(_) => {
                                    unrouted.push(json!({"target":dest,"reason":"event_id_out_of_range"}));
                                    continue;
                                }
                            };
                            let reply = self
                                .relay(
                                    address,
                                    &Message::Backward {
                                        target: dest,
                                        event_id: parent_event,
                                        from: next_source,
                                        gradient: core_packet.gradient,
                                        ttl_ms: remaining,
                                        gradient_hops: core_packet.hops_left,
                                        route_hops: 4,
                                    },
                                )
                                .await;
                            if reply["kind"] == "backward_result"
                                && (reply["status"] == "applied" || reply["status"] == "ignored_duplicate"
                                    || reply["status"] == "pending")
                            {
                                if let Some(extra) = reply["unrouted"].as_array() {
                                    unrouted.extend(extra.iter().cloned());
                                }
                            } else {
                                unrouted.push(json!({
                                    "target":dest,
                                    "reason":"delivery_not_confirmed",
                                    "response":reply
                                }));
                            }
                        }
                        json!({
                            "kind":"backward_result",
                            "status":"applied",
                            "version":version,
                            "unrouted":unrouted
                        })
                    }
                }
            }
        }
    }

    async fn relay(&self, address: SocketAddr, msg: &Message) -> Value {
        let request_value = match serde_json::to_value(msg) {
            Ok(value) => value,
            Err(_) => return error_response("message_encoding_error"),
        };
        match request(address, &request_value).await {
            Ok(reply) => reply,
            Err(_) => error_response("peer_delivery_uncertain"),
        }
    }
}

fn remaining_ms(deadline: Instant) -> u64 {
    let remaining = deadline.saturating_duration_since(Instant::now()).as_millis();
    u64::try_from(remaining).unwrap_or(u64::MAX)
}

fn error_response(code: &str) -> Value {
    json!({"kind":"error","code":code})
}

async fn handle_connection(mut stream: TcpStream, state: Arc<NodeState>) -> io::Result<()> {
    let incoming = timeout(MAX_IO_WAIT, read_frame(&mut stream))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "frame timed out"))??;
    let reply = match serde_json::from_value::<Message>(incoming) {
        Ok(message) => state.process(message).await,
        Err(_) => error_response("invalid_protocol_message"),
    };
    timeout(MAX_IO_WAIT, write_frame(&mut stream, &reply))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "response timed out"))?
}

async fn gossip_loop(state: Arc<NodeState>) {
    if state.peers.is_empty() {
        return;
    }
    let peers: Vec<_> = state.peers.values().copied().collect();
    let mut index = 0_usize;
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    loop {
        tick.tick().await;
        let routes: Vec<_> = state
            .routes
            .read()
            .await
            .iter()
            .take(MAX_ADVERTISED_ROUTES)
            .map(|(neuron, route)| Advert {
                owner: route.owner,
                neuron: *neuron,
                epoch: route.epoch,
            })
            .collect();
        let frame = json!({
            "kind":"gossip",
            "from_node":state.id,
            "routes":routes
        });
        let _ = request(peers[index % peers.len()], &frame).await;
        index = index.wrapping_add(1);
    }
}

/// Run one neuron owner, listening on a dedicated TCP address. Multiple
/// processes form a trusted development cluster through bounded gossip.
pub async fn serve(config: NodeConfig) -> io::Result<()> {
    if config.id == 0
        || config.neuron.id() == 0
        || !is_loopback(config.address.ip())
        || config.peers.len() > MAX_ADVERTISED_ROUTES
    {
        return Err(invalid("invalid node identity, address or peer count"));
    }
    let mut peers = BTreeMap::new();
    for peer in config.peers {
        if peer.id == 0
            || peer.id == config.id
            || !is_loopback(peer.address.ip())
            || peers.insert(peer.id, peer.address).is_some()
        {
            return Err(invalid("invalid or duplicate bootstrap peer"));
        }
    }
    let mut routes = BTreeMap::new();
    routes.insert(
        config.neuron.id(),
        Route {
            owner: config.id,
            epoch: 1,
        },
    );
    let state = Arc::new(NodeState {
        id: config.id,
        neuron_id: config.neuron.id(),
        neuron: Mutex::new(config.neuron),
        peers,
        routes: RwLock::new(routes),
    });
    let listener = TcpListener::bind(config.address).await?;
    let slots = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));
    tokio::spawn(gossip_loop(Arc::clone(&state)));
    loop {
        // Acquire before accept so overload reaches the TCP backlog rather
        // than spawning unbounded tasks.
        let permit = Arc::clone(&slots)
            .acquire_owned()
            .await
            .map_err(|_| invalid("connection pool closed"))?;
        let (socket, _) = listener.accept().await?;
        let node = Arc::clone(&state);
        tokio::spawn(async move {
            let _permit = permit;
            let _ = handle_connection(socket, node).await;
        });
    }
}

fn is_loopback(ip: IpAddr) -> bool {
    ip.is_loopback()
}
