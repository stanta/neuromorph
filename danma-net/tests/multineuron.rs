//! Four training activations across six neurons owned by three TCP processes.
//! Two neurons in each process share a fixed, two-worker CPU shard.
use danma_net::request;
use serde_json::{json, Value};
use std::{
    net::{SocketAddr, TcpListener},
    process::{Child, Command, Stdio},
    time::Duration,
};
use tokio::time::{sleep, timeout};

struct Nodes(Vec<Child>);
impl Drop for Nodes {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

async fn call(address: SocketAddr, message: Value) -> Value {
    request(address, &message).await.unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn six_neurons_on_three_processes_train_through_local_and_remote_edges() {
    let mut ports = Vec::new();
    while ports.len() < 3 {
        let p = port();
        if !ports.contains(&p) {
            ports.push(p);
        }
    }
    let addresses: Vec<SocketAddr> = ports
        .iter()
        .map(|p| format!("127.0.0.1:{p}").parse().unwrap())
        .collect();
    let layouts: [(u64, &str, u64, &str); 3] = [
        (1, "99:2", 4, "1:1"),
        (2, "1:3", 5, "4:4"),
        (3, "2:4", 6, "5:5"),
    ];
    let mut processes = Vec::new();
    for (index, (first, first_weight, second, second_weight)) in layouts.into_iter().enumerate() {
        let id = (index + 1) as u64;
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_danma-node"));
        cmd.args([
            "--id", &id.to_string(),
            "--listen", &addresses[index].to_string(),
            "--workers", "2", "--mailbox", "16",
            "--neuron", &first.to_string(), "--weight", first_weight,
            "--neuron", &second.to_string(), "--weight", second_weight,
        ]);
        for (peer_index, address) in addresses.iter().enumerate() {
            if peer_index != index {
                cmd.args(["--peer", &format!("{}@{address}", peer_index + 1)]);
            }
        }
        processes.push(cmd.stdout(Stdio::null()).stderr(Stdio::inherit()).spawn().unwrap());
    }
    let _nodes = Nodes(processes);

    timeout(Duration::from_secs(12), async {
        loop {
            let mut ready = true;
            for address in &addresses {
                match request(*address, &json!({"kind":"routes"})).await {
                    Ok(result) if [1_u64, 2, 3, 4, 5, 6].iter().all(|id| {
                        let owner = match id { 1 | 4 => 1, 2 | 5 => 2, _ => 3 };
                        result["routes"][id.to_string()] == owner
                    }) => {}
                    _ => ready = false,
                }
            }
            if ready { break; }
            sleep(Duration::from_millis(50)).await;
        }
    }).await.expect("six neuron routes did not converge");

    let entry = addresses[0];
    let cases = [
        (1_u64, 10_u64, 99_u64, 1_u64, 1.0, json!({"kind":"neuron","neuron_id":4,"event_id":40}), 2.0),
        (4, 40, 1, 10, 2.0, json!({"kind":"neuron","neuron_id":5,"event_id":50}), 2.0),
        (5, 50, 4, 40, 2.0, json!({"kind":"neuron","neuron_id":6,"event_id":60}), 8.0),
        (6, 60, 5, 50, 8.0, json!({"kind":"teacher"}), 40.0),
    ];
    for (target, event_id, from, source_event_id, value, expected, output) in cases {
        let result = call(entry, json!({
            "kind":"forward","target":target,"event_id":event_id,"trace_id":77,
            "route_hops":4,"inputs":[{"from":from,"source_event_id":source_event_id,"value":value}],
            "expected":[expected]
        })).await;
        assert_eq!(result["kind"], "forward_result", "forward for {target}: {result}");
        assert_eq!(result["output"], output);
    }

    let teacher = json!({
        "kind":"backward","target":6,"event_id":60,"from":{"kind":"teacher"},
        "gradient":40.0,"ttl_ms":3000,"gradient_hops":6,"route_hops":4
    });
    let trained = call(entry, teacher.clone()).await;
    assert_eq!(trained["kind"], "backward_result", "{trained}");
    assert_eq!(trained["status"], "applied", "{trained}");
    assert_eq!(trained["unrouted"][0]["target"], 99, "{trained}");
    for id in [1_u64, 4, 5, 6] {
        let state = call(entry, json!({"kind":"inspect","target":id,"route_hops":4})).await;
        assert_eq!(state["version"], 1, "neuron {id}: {state}");
    }
    for id in [2_u64, 3] {
        let state = call(entry, json!({"kind":"inspect","target":id,"route_hops":4})).await;
        assert_eq!(state["version"], 0, "unused neuron {id}: {state}");
    }
    let replay = call(entry, teacher).await;
    assert_eq!(replay["status"], "ignored_duplicate");
    let checked = call(entry, json!({"kind":"inspect","target":1,"route_hops":4})).await;
    assert_eq!(checked["version"], 1);
}
