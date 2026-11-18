//! Manual check against a running Discord: shows a presence for a few seconds, then clears it.
//! Usage: cargo run -p decoy-core --example live -- <app_id> [seconds]
use decoy_core::ipc::{default_paths, read_frame, write_frame, OP_FRAME, OP_HANDSHAKE};
use serde_json::json;
use std::os::unix::net::UnixStream;
use std::time::Duration;

fn main() {
    let mut args = std::env::args().skip(1);
    let app_id = args.next().expect("app id");
    let secs: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(15);
    let path = default_paths()
        .into_iter()
        .find(|p| p.exists())
        .expect("no discord-ipc socket");
    println!("socket: {}", path.display());
    let mut s = UnixStream::connect(&path).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    write_frame(
        &mut s,
        OP_HANDSHAKE,
        &serde_json::to_vec(&json!({"v":1,"client_id":app_id})).unwrap(),
    )
    .unwrap();
    let (op, ready) = read_frame(&mut s).unwrap();
    let ready: serde_json::Value = serde_json::from_slice(&ready).unwrap();
    println!(
        "handshake reply op={op}: {}",
        serde_json::to_string(&ready)
            .unwrap()
            .chars()
            .take(500)
            .collect::<String>()
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let activity = json!({
        "type": 3, "name": "Decoy test", "details": "Testing Decoy", "state": "Ignore this, back in a moment",
        "status_display_type": 2,
        "timestamps": {"start": now},
        "party": {"id": "decoy-party", "size": [2, 4]},
        "buttons": [{"label": "Example", "url": "https://example.com"}],
        "instance": false
    });
    write_frame(&mut s, OP_FRAME, &serde_json::to_vec(&json!({"cmd":"SET_ACTIVITY","args":{"pid":std::process::id(),"activity":activity},"nonce":"live-1"})).unwrap()).unwrap();
    let (op, resp) = read_frame(&mut s).unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&resp).unwrap();
    println!(
        "set reply op={op}: {}",
        serde_json::to_string_pretty(&resp).unwrap()
    );
    std::thread::sleep(Duration::from_secs(secs));
    write_frame(&mut s, OP_FRAME, &serde_json::to_vec(&json!({"cmd":"SET_ACTIVITY","args":{"pid":std::process::id(),"activity":null},"nonce":"live-2"})).unwrap()).unwrap();
    let (op, resp) = read_frame(&mut s).unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&resp).unwrap();
    println!(
        "clear reply op={op}: {}",
        serde_json::to_string(&resp)
            .unwrap()
            .chars()
            .take(300)
            .collect::<String>()
    );
}
