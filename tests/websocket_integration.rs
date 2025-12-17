use futures_util::{SinkExt, StreamExt};
use std::net::TcpListener as StdTcpListener;
use std::process::{Child, Command, Stdio};
use tokio::time::{sleep, timeout, Duration};
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;

fn pick_free_local_port() -> u16 {
    // Bind to port 0 to let the OS pick an available port, then immediately release it.
    // There is a small race window, but it's typically acceptable for tests.
    let listener = StdTcpListener::bind(("127.0.0.1", 0)).expect("failed to bind ephemeral port");
    listener.local_addr().expect("failed to read local addr").port()
}

fn spawn_server(bind_addr: &str) -> Child {
    let exe = env!("CARGO_BIN_EXE_neuromorph");
    Command::new(exe)
        .env("NEUROMORPH_WS_ADDR", bind_addr)
        // Keep output quiet in tests; enable temporarily if debugging flakes.
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn neuromorph server")
}

#[tokio::test]
async fn websocket_end_to_end_roundtrip() {
    // Arrange
    let port = pick_free_local_port();
    let bind_addr = format!("127.0.0.1:{port}");
    let ws_url = format!("ws://127.0.0.1:{port}");

    let mut child = spawn_server(&bind_addr);

    // Act: wait for server to come up by retrying the WebSocket connection.
    let connect_result = timeout(Duration::from_secs(5), async {
        loop {
            match connect_async(&ws_url).await {
                Ok((ws_stream, _)) => break ws_stream,
                Err(_) => sleep(Duration::from_millis(50)).await,
            }
        }
    })
    .await;

    let ws_stream = match connect_result {
        Ok(ws_stream) => ws_stream,
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            panic!("timed out waiting for server to accept WebSocket connections");
        }
    };

    let (mut write, mut read) = ws_stream.split();
    write
        .send(Message::Text("1,2,3".to_string()))
        .await
        .expect("failed to send websocket message");

    let msg = timeout(Duration::from_secs(3), async { read.next().await })
        .await
        .expect("timed out waiting for response")
        .expect("websocket stream ended")
        .expect("websocket read error");

    let text = msg.to_text().expect("expected text response");
    let parsed: i32 = text.parse().expect("response was not an i32");

    // Assert: we got a valid response (via handle_client -> Neuron::forward).
    assert!(parsed >= 0);

    // Cleanup
    let _ = child.kill();
    let _ = child.wait();
}

