//! Manual check against a running Discord: shows a presence for a few seconds, then clears it.
//! Usage: cargo run -p decoy-core --example live -- <app_id> [seconds]
use decoy_core::{Client, ConnectOptions};
use serde_json::json;
use std::time::{Duration, Instant};

fn main() {
    let mut args = std::env::args().skip(1);
    let app_id = args.next().expect("usage: live <app_id> [seconds]");
    let secs: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(15);

    let (mut client, user) =
        Client::connect(&app_id, &ConnectOptions::default()).expect("could not reach Discord");
    println!("connected as {user:?}");

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
    client.set_activity(Some(&activity)).expect("SET_ACTIVITY");
    println!("presence set, holding it for {secs} s");

    let until = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < until {
        client.poll().expect("connection lost");
        std::thread::sleep(Duration::from_millis(500));
    }
    client.set_activity(None).expect("clear");
    println!("presence cleared");
}
