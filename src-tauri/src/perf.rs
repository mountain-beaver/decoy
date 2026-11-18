//! Self-tests, never active in normal runs.
//!
//! * `DECOY_PERF=1`: the window measures how many frames it really draws (idle
//!   and while scrolling the form), prints one JSON line to stdout and exits.
//! * `DECOY_SMOKE=<file>`: the window reports that it rendered, presses its
//!   main controls and checks that each one works; the report is written to
//!   the file and the app exits.
//!   CI runs this on Windows, macOS and Linux and takes a screenshot meanwhile.

use tauri::AppHandle;

pub fn enabled() -> bool {
    std::env::var_os("DECOY_PERF").is_some()
}

/// A test instance that must not hand over to the user's running one:
/// the self-test, or a run with `DECOY_DEV_QUERY` (start on a given screen).
pub fn isolated() -> bool {
    enabled()
        || smoke_file().is_some()
        || std::env::var_os("DECOY_DEV_QUERY").is_some()
        || std::env::var_os("DECOY_SMOKE_EDIT").is_some()
}

/// Where the smoke test writes its report; `None` in normal runs.
pub fn smoke_file() -> Option<std::path::PathBuf> {
    std::env::var_os("DECOY_SMOKE")
        .filter(|v| !v.is_empty())
        .map(Into::into)
}

/// The script to inject before the page's own, if a self-test is on.
pub fn script() -> Option<&'static str> {
    if enabled() {
        Some(SCRIPT)
    } else if std::env::var_os("DECOY_SMOKE_EDIT").is_some() {
        Some(EDIT_SCRIPT)
    } else if smoke_file().is_some() {
        Some(SMOKE_SCRIPT)
    } else {
        None
    }
}

/// Smoke test: give up when the window never reports.
pub fn start_watchdog() {
    let Some(file) = smoke_file() else { return };
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(90));
        if !file.exists() {
            let _ = std::fs::write(
                &file,
                r#"{"ok":false,"error":"no report from the window in 90 s"}"#,
            );
            std::process::exit(1);
        }
    });
}

const SMOKE_SCRIPT: &str = r#"
(function () {
  if (window.top !== window) return;
  const t0 = performance.now();
  const errors = [];
  window.addEventListener('error', (e) => errors.push(String(e.message)));
  window.addEventListener('unhandledrejection', (e) => errors.push(String(e.reason)));
  const invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const send = (r) => invoke('smoke_report', { report: JSON.stringify(r) });
  const wait = (ms) => new Promise((r) => setTimeout(r, ms));
  const until = async (check, ms) => {
    const start = performance.now();
    while (performance.now() - start < (ms || 4000)) {
      if (await check()) return true;
      await wait(100);
    }
    return false;
  };
  const byText = (selector, text) =>
    [...document.querySelectorAll(selector)].find((e) => e.textContent.trim() === text);
  // Is the control really on top at its own centre, or is something covering it?
  const onTop = (el) => {
    const r = el.getBoundingClientRect();
    const at = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    return !!at && (at === el || el.contains(at));
  };

  // Press the main controls and check that each one does its job.
  const pressControls = async () => {
    const clicks = [];
    const press = async (name, el, check) => {
      if (!el) return clicks.push({ name, ok: false, why: 'control not found' });
      if (el.disabled) return clicks.push({ name, ok: false, why: 'control is disabled' });
      const top = onTop(el);
      el.click();
      clicks.push({ name, ok: top && (await until(check)), on_top: top });
    };
    const kind = () => (document.querySelector('.dc-kind') || {}).textContent || '';
    const showing = async () => (await invoke('get_state')).showing;
    await press('guide opens', byText('button', 'Guide'), () => !!byText('button', 'Back to editor'));
    await press('guide closes', byText('button', 'Back to editor'), () => !!byText('button', 'Guide'));
    await press('activity kind changes', byText('[role=radiogroup] label', 'Watching'), () => /Watching/.test(kind()));
    await press('activity kind changes back', byText('[role=radiogroup] label', 'Playing'), () => /Playing/.test(kind()));
    const sw = document.querySelector('button.switch[role=switch]');
    await press('sharing switches on', sw, async () => (await showing()) === true);
    await press('sharing switches off', sw, async () => (await showing()) === false);
    return clicks;
  };

  const report = async (text) => {
    try {
      const state = await invoke('get_state');
      const clicks = await pressControls();
      const fonts = document.fonts ? [...document.fonts].filter((f) => f.status === 'loaded').length : -1;
      send({
        ok: errors.length === 0 && clicks.every((c) => c.ok),
        ms: Math.round(performance.now() - t0), ua: navigator.userAgent,
        dpr: devicePixelRatio, width: innerWidth, height: innerHeight, fonts,
        profiles: (state.profiles || []).length, engine: state.engine && state.engine.status,
        clicks, text: text.replace(/\s+/g, ' ').slice(0, 160), errors,
      });
    } catch (e) {
      send({ ok: false, error: String(e), errors });
    }
  };
  const tick = () => {
    const text = ((document.body && document.body.innerText) || '').trim();
    if (text.length > 30 && window.__TAURI_INTERNALS__) {
      // Let fonts and the first network answers settle first.
      setTimeout(() => report(text), 1500);
    } else if (performance.now() - t0 > 60000) {
      send({ ok: false, error: 'the page did not render', text, errors });
    } else {
      setTimeout(tick, 250);
    }
  };
  tick();
})();
"#;

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

/// Smoke test: store the window's report, keep the window up for the
/// screenshot (`DECOY_SMOKE_HOLD` seconds), then exit.
#[tauri::command]
pub fn smoke_report(app: AppHandle, report: String) {
    let Some(file) = smoke_file() else { return };
    let ok = report.contains("\"ok\":true");
    let _ = std::fs::write(&file, &report);
    let hold = std::env::var("DECOY_SMOKE_HOLD")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(hold));
        app.exit(if ok { 0 } else { 1 });
    });
}

/// Self-test of saving on close (`DECOY_SMOKE_EDIT=1`): type into the first
/// text field and ask for the window to close in the same instant, well inside
/// the page's save delay. The caller then looks for the text in profiles.json.
const EDIT_SCRIPT: &str = r#"
(() => {
  const started = Date.now();
  const tick = () => {
    const input = document.querySelector('.field input[type=text]');
    if (!input || !window.__TAURI_INTERNALS__) {
      if (Date.now() - started < 30000) setTimeout(tick, 100);
      return;
    }
    setTimeout(() => {
      input.value = 'typed right before closing';
      input.dispatchEvent(new Event('input', { bubbles: true }));
      window.__TAURI_INTERNALS__.invoke('smoke_close');
    }, 1500);
  };
  tick();
})();
"#;

/// Self-test: ask for the window to close the way its close button does.
#[tauri::command]
pub fn smoke_close(window: tauri::WebviewWindow) {
    if std::env::var_os("DECOY_SMOKE_EDIT").is_some() {
        let _ = window.close();
    }
}
