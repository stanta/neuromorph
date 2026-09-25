//! Three independently spawned TCP processes; not an in-process simulation.
use danma_net::request;
use serde_json::{json, Value};
use std::net::{SocketAddr, TcpListener};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::{sleep, timeout};

fn free_port() -> u16 {
    let socket = TcpListener::bind("127.0.0.1:0").expect("bind test port");
    socket.local_addr().expect("local address").port()
}

struct Children(Vec<Child>);

impl Drop for Children {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn child(id: u64, addr: SocketAddr, neuron: u64, weight: &str, others: &[(u64, SocketAddr)]) -> Child {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_danma-node"));
    cmd.args(["--id", &id.to_string(), "--listen", &addr.to_string(),
        "--neuron", &neuron.to_string(), "--weight", weight]);
    for (peer_id, peer_addr) in others {
        cmd.args(["--peer", &format!("{peer_id}@{peer_addr}")]);
    }
    cmd.stdout(Stdio::null()).stderr(Stdio::inherit());
    cmd.spawn().expect("spawn DANMA node")
}

async fn call(addr: SocketAddr, req: Value) -> Value {
    request(addr, &req).await.expect("valid DANMA response")
}

async fn cluster() -> (Children, [SocketAddr; 3]) {
    let addresses = [
        format!("127.0.0.1:{}", free_port()).parse().unwrap(),
        format!("127.0.0.1:{}", free_port()).parse().unwrap(),
        format!("127.0.0.1:{}", free_port()).parse().unwrap(),
    ];
    let children = Children(vec![
        child(1, addresses[0], 1, "99:2", &[(2, addresses[1]), (3, addresses[2])]),
        child(2, addresses[1], 2, "1:3", &[(1, addresses[0]), (3, addresses[2])]),
        child(3, addresses[2], 3, "2:4", &[(1, addresses[0]), (2, addresses[1])]),
    ]);
    timeout(Duration::from_secs(12), async {
        loop {
            let mut converged = true;
            for address in addresses {
                match request(address, &json!({"kind":"routes"})).await {
                    Ok(response) if response["kind"] == "routes_result"
                        && response["routes"]["1"] == 1
                        && response["routes"]["2"] == 2
                        && response["routes"]["3"] == 3 => {}
                    _ => converged = false,
                }
            }
            if converged {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    }).await.expect("three nodes did not exchange gossip routes");
    (children, addresses)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn three_processes_forward_backward_and_deduplicate() {
    let (_children, addr) = cluster().await;
    let entry = addr[0];
    let a = call(entry, json!({
        "kind":"forward", "target":1, "event_id":10, "trace_id":7,
        "route_hops":4,
        "inputs":[{"from":99,"source_event_id":1,"value":1.0}],
        "expected":[{"kind":"neuron","neuron_id":2,"event_id":20}]
    })).await;
    assert_eq!(a["kind"], "forward_result");
    assert_eq!(a["output"], 2.0);

    let b = call(entry, json!({
        "kind":"forward", "target":2, "event_id":20, "trace_id":7,
        "route_hops":4,
        "inputs":[{"from":1,"source_event_id":10,"value":2.0}],
        "expected":[{"kind":"neuron","neuron_id":3,"event_id":30}]
    })).await;
    assert_eq!(b["output"], 6.0);
    let c = call(entry, json!({
        "kind":"forward", "target":3, "event_id":30, "trace_id":7,
        "route_hops":4,
        "inputs":[{"from":2,"source_event_id":20,"value":6.0}],
        "expected":[{"kind":"teacher"}]
    })).await;
    assert_eq!(c["output"], 24.0);

    let feedback = json!({
        "kind":"backward","target":3,"event_id":30,
        "from":{"kind":"teacher"},"gradient":24.0,
        "ttl_ms":3000,"gradient_hops":5,"route_hops":4
    });
    let trained = call(entry, feedback.clone()).await;
    assert_eq!(trained["kind"], "backward_result");
    assert_eq!(trained["status"], "applied");
    assert_eq!(trained["version"], 1);
    // Host-input gradient must be returned, never silently discarded.
    assert_eq!(trained["unrouted"][0]["target"], 99);

    for neuron in [1_u64, 2, 3] {
        let inspected = call(entry, json!({
            "kind":"inspect", "target":neuron, "route_hops":4
        })).await;
        assert_eq!(inspected["kind"], "inspect_result");
        assert_eq!(inspected["version"], 1, "neuron {neuron} did not learn");
    }
    let replay = call(entry, feedback).await;
    assert_eq!(replay["status"], "ignored_duplicate");
    let inspected = call(entry, json!({
        "kind":"inspect","target":3,"route_hops":4
    })).await;
    assert_eq!(inspected["version"], 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_and_forged_feedback_cannot_train() {
    let (_children, addr) = cluster().await;
    let entry = addr[0];
    let created = call(entry, json!({
        "kind":"forward","target":3,"event_id":31,"trace_id":8,"route_hops":4,
        "inputs":[{"from":2,"source_event_id":20,"value":1.0}],
        "expected":[{"kind":"teacher"}]
    })).await;
    assert_eq!(created["output"], 4.0);

    let expired = call(entry, json!({
        "kind":"backward","target":3,"event_id":31,
        "from":{"kind":"teacher"},"gradient":1.0,
        "ttl_ms":0,"gradient_hops":4,"route_hops":4
    })).await;
    assert_eq!(expired["status"], "expired");

    let forged = call(entry, json!({
        "kind":"backward","target":3,"event_id":31,
        "from":{"kind":"neuron","neuron_id":99,"event_id":7},"gradient":1.0,
        "ttl_ms":2000,"gradient_hops":4,"route_hops":4
    })).await;
    assert_eq!(forged["kind"], "error");
    let untouched = call(entry, json!({
        "kind":"inspect","target":3,"route_hops":4
    })).await;
    assert_eq!(untouched["version"], 0);
    let trained = call(entry, json!({
        "kind":"backward","target":3,"event_id":31,
        "from":{"kind":"teacher"},"gradient":1.0,
        "ttl_ms":2000,"gradient_hops":1,"route_hops":4
    })).await;
    assert_eq!(trained["status"], "applied");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_target_is_a_reported_error() {
    let (_children, addr) = cluster().await;
    let result = call(addr[0], json!({
        "kind":"inspect","target":999,"route_hops":4
    })).await;
    assert_eq!(result["kind"], "error");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_gossip_batch_does_not_publish_a_partial_route() {
    let (_children, addr) = cluster().await;
    // A valid advertisement followed by an invalid owner must not leave
    // neuron 4 routable. Gossip is a control-plane atomic update.
    let reply = call(addr[0], json!({
        "kind":"gossip", "from_node":2,
        "routes":[
            {"owner":2,"neuron":4,"epoch":2},
            {"owner":777,"neuron":5,"epoch":2}
        ]
    })).await;
    assert_eq!(reply["kind"], "error");
    let routes = call(addr[0], json!({"kind":"routes"})).await;
    assert!(routes["routes"].get("4").is_none(), "invalid batch installed a route");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn oversized_frame_does_not_stop_the_node() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (_children, addr) = cluster().await;
    let mut socket = tokio::net::TcpStream::connect(addr[0]).await.unwrap();
    socket.write_u32(65_537).await.unwrap();
    // The length alone exceeds the 64 KiB bound. The node closes this
    // connection without allocating a user-specified buffer.
    let mut byte = [0_u8; 1];
    let read = timeout(Duration::from_secs(2), socket.read(&mut byte)).await;
    assert!(matches!(read, Ok(Ok(0)) | Ok(Err(_))));
    let alive = call(addr[0], json!({"kind":"routes"})).await;
    assert_eq!(alive["kind"], "routes_result");
}
