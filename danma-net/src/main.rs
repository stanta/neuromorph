use danma_core::{Activation, Config, Neuron};
use danma_net::{serve, NodeConfig, Peer};
use std::{env, net::SocketAddr, process};

fn parse_flags() -> Result<NodeConfig, String> {
    let mut args = env::args().skip(1);
    let mut id: Option<u64> = None;
    let mut listen: Option<SocketAddr> = None;
    let mut specs: Vec<(u64, Vec<(u64, f32)>)> = Vec::new();
    let mut worker_threads: usize = 2;
    let mut mailbox_capacity: usize = 256;
    let mut peers = Vec::new();

    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--id" => {
                id = Some(value.parse().map_err(|_| "invalid node ID")?);
            }
            "--listen" => {
                listen = Some(value.parse().map_err(|_| "invalid TCP address")?);
            }
            "--neuron" => {
                specs.push((value.parse().map_err(|_| "invalid neuron ID")?, Vec::new()));
            }
            "--weight" => {
                let (from, weight) = value
                    .split_once(':')
                    .ok_or("weight must have format source:weight")?;
                let spec = specs
                    .last_mut()
                    .ok_or("--weight requires a preceding --neuron")?;
                spec.1.push((
                    from.parse().map_err(|_| "invalid source neuron")?,
                    weight.parse().map_err(|_| "invalid weight")?,
                ));
            }
            "--workers" => {
                worker_threads = value.parse().map_err(|_| "invalid worker count")?;
            }
            "--mailbox" => {
                mailbox_capacity = value.parse().map_err(|_| "invalid mailbox capacity")?;
            }
            "--peer" => {
                let (peer_id, addr) = value
                    .split_once('@')
                    .ok_or("peer must have format node_id@host:port")?;
                peers.push(Peer {
                    id: peer_id.parse().map_err(|_| "invalid peer ID")?,
                    address: addr.parse().map_err(|_| "invalid peer TCP address")?,
                });
            }
            _ => return Err(format!("unknown flag: {flag}")),
        }
    }
    let node_id = id.ok_or("--id is required")?;
    let address = listen.ok_or("--listen is required")?;
    if specs.is_empty() {
        return Err("at least one --neuron is required".into());
    }
    let neurons: Vec<Neuron> = specs
        .into_iter()
        .map(|(neuron_id, weights)| {
            Neuron::new(
                neuron_id,
                0.0,
                weights,
                Config {
                    activation: Activation::Linear,
                    learning_rate: 0.1,
                    activation_ttl_ms: 10_000,
                    replay_retention_ms: 10_000,
                    max_live_events: 4_096,
                    max_staleness_versions: 8,
                },
            )
            .map_err(|error| format!("invalid neuron config for {neuron_id}: {error:?}"))
        })
        .collect::<Result<_, _>>()?;

    Ok(NodeConfig {
        id: node_id,
        address,
        neurons,
        worker_threads,
        mailbox_capacity,
        peers,
    })
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let config = match parse_flags() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("DANMA node configuration: {error}");
            eprintln!(
                "Usage: danma-node --id N --listen 127.0.0.1:PORT \
                 [--workers N] [--mailbox N] \
                 --neuron ID [--weight SOURCE:WEIGHT]... \
                 [--neuron ID --weight SOURCE:WEIGHT]... \
                 [--peer NODE_ID@127.0.0.1:PORT]..."
            );
            process::exit(2);
        }
    };
    if let Err(error) = serve(config).await {
        eprintln!("DANMA node stopped: {error}");
        process::exit(1);
    }
}
