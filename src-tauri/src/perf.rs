//! Frame-rate self-test, enabled with `DECOY_PERF=1`. The window measures how
//! many frames it really draws (idle and while scrolling the form), prints one
//! JSON line to stdout and exits. Used to check smoothness on high refresh
//! rate displays; never active in normal runs.

use tauri::AppHandle;

pub fn enabled() -> bool {
    std::env::var_os("DECOY_PERF").is_some()
}

/// A test instance that must not hand over to the user's running one:
/// the self-test, or a run with `DECOY_DEV_QUERY` (start on a given screen).
pub fn isolated() -> bool {
    enabled() || std::env::var_os("DECOY_DEV_QUERY").is_some()
}

/// `DECOY_DEV_SIZE=1900x1250` opens the window at that size (for the self-test).
pub fn window_size() -> (f64, f64) {
    std::env::var("DECOY_DEV_SIZE")
        .ok()
        .and_then(|v| {
            let (w, h) = v.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or((920.0, 640.0))
}

/// The page to load; `DECOY_DEV_QUERY="open=guide&chapter=2"` adds a query string.
pub fn start_page() -> String {
    match std::env::var("DECOY_DEV_QUERY") {
        Ok(q) if !q.is_empty() => format!("index.html?{q}"),
        _ => "index.html".into(),
    }
}

/// Injected before the page's own scripts when the self-test is on.
pub const SCRIPT: &str = r#"
window.addEventListener('load', () => setTimeout(async () => {
  const run = (ms, step) => new Promise((done) => {
    const t = []; let start;
    const f = (now) => {
      start ??= now; t.push(now);
      if (step) step(now - start);
      if (now - start < ms) requestAnimationFrame(f); else done(t);
    };
    requestAnimationFrame(f);
  });
  const stats = (t) => {
    const d = []; for (let i = 1; i < t.length; i++) d.push(t[i] - t[i - 1]);
    d.sort((a, b) => a - b);
    const at = (q) => +d[Math.min(d.length - 1, Math.floor(d.length * q))].toFixed(2);
    return { fps: +(1000 * d.length / (t[t.length - 1] - t[0])).toFixed(1), median_ms: at(0.5), p95_ms: at(0.95), p99_ms: at(0.99), worst_ms: at(1) };
  };
  const idle = stats(await run(3000));
  const box = [...document.querySelectorAll('*')]
    .filter((e) => e.scrollHeight > e.clientHeight + 40 && /auto|scroll/.test(getComputedStyle(e).overflowY))
    .sort((a, b) => b.clientHeight - a.clientHeight)[0];
  let scroll = null;
  if (box) {
    const max = box.scrollHeight - box.clientHeight;
    scroll = stats(await run(4500, (ms) => { const p = (ms % 1500) / 1500; box.scrollTop = max * (p < 0.5 ? p * 2 : 2 - p * 2); }));
    scroll.distance_px = max;
  }
  const report = { dpr: devicePixelRatio, width: innerWidth, height: innerHeight, idle, scroll };
  window.__TAURI_INTERNALS__.invoke('perf_report', { report: JSON.stringify(report) });
}, 2500));
"#;

#[tauri::command]
pub fn perf_report(app: AppHandle, report: String) {
    println!("DECOY_PERF {report}");
    app.exit(0);
}
