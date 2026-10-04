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

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn spawn_node(
    id: u64,
    address: SocketAddr,
    neuron: u64,
    weight: &str,
    axon: Option<&str>,
    peers: &[(u64, SocketAddr)],
) -> Child {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_danma-node"));
    cmd.args([
        "--id",
        &id.to_string(),
        "--listen",
        &address.to_string(),
        "--neuron",
        &neuron.to_string(),
        "--weight",
        weight,
    ]);
    if let Some(axon) = axon {
        cmd.args(["--axon", axon]);
    }
    for (peer_id, peer_address) in peers {
        cmd.args(["--peer", &format!("{peer_id}@{peer_address}")]);
    }
    cmd.stdout(Stdio::null()).stderr(Stdio::inherit());
    cmd.spawn().expect("spawn DANMA node")
}

async fn call(address: SocketAddr, message: Value) -> Value {
    request(address, &message).await.expect("valid DANMA response")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn forward_propagates_through_axons_across_three_processes() {
    let mut ports = Vec::new();
    while ports.len() < 3 {
        let port = free_port();
        if !ports.contains(&port) {
            ports.push(port);
        }
    }
    let addresses = [
        format!("127.0.0.1:{}", ports[0]).parse().unwrap(),
        format!("127.0.0.1:{}", ports[1]).parse().unwrap(),
        format!("127.0.0.1:{}", ports[2]).parse().unwrap(),
    ];

    let processes = vec![
        spawn_node(
            1,
            addresses[0],
            1,
            "99:2",
            Some("12:2"),
            &[(2, addresses[1]), (3, addresses[2])],
        ),
        spawn_node(
            2,
            addresses[1],
            2,
            "1:3",
            Some("23:3"),
            &[(1, addresses[0]), (3, addresses[2])],
        ),
        spawn_node(
            3,
            addresses[2],
            3,
            "2:4",
            None,
            &[(1, addresses[0]), (2, addresses[1])],
        ),
    ];
    let _nodes = Nodes(processes);

    timeout(Duration::from_secs(12), async {
        loop {
            let mut ready = true;
            for address in addresses {
                match request(address, &json!({"kind":"routes"})).await {
                    Ok(result)
                        if result["routes"]["1"] == 1
                            && result["routes"]["2"] == 2
                            && result["routes"]["3"] == 3 => {}
                    _ => ready = false,
                }
            }
            if ready {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("axon routes did not converge");

    let result = call(
        addresses[0],
        json!({
            "kind":"forward",
            "target":1,
            "event_id":10,
            "trace_id":77,
            "route_hops":4,
            "forward_hops":8,
            "inputs":[{"from":99,"source_event_id":1,"value":1.0}],
            "expected":[]
        }),
    )
    .await;

    assert_eq!(result["kind"], "forward_result", "{result}");
    assert_eq!(result["output"], 2.0, "{result}");
    assert_eq!(result["terminals"].as_array().unwrap().len(), 1, "{result}");
    assert_eq!(result["terminals"][0]["neuron"], 3, "{result}");
    assert_eq!(result["terminals"][0]["output"], 24.0, "{result}");
    assert!(result["unrouted"].as_array().unwrap().is_empty(), "{result}");

    let first = call(
        addresses[0],
        json!({"kind":"inspect","target":1,"route_hops":4}),
    )
    .await;
    assert_eq!(first["axons"][0]["edge_id"], 12);
    assert_eq!(first["axons"][0]["to"], 2);

    let third = call(
        addresses[0],
        json!({"kind":"inspect","target":3,"route_hops":4}),
    )
    .await;
    assert!(third["axons"].as_array().unwrap().is_empty());
}
