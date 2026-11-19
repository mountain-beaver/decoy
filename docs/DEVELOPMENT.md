# Building and working on Decoy

Decoy is a Tauri 2 app: a Rust core, a small Svelte window, and on Linux a separate tray process.

## Build from source

You need Rust, Node.js with pnpm, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your system.

```sh
pnpm install
pnpm tauri build
```

With Nix, run `nix develop` and build with `cargo tauri build` instead.

## Project layout

| Path | What it is |
|---|---|
| `crates/decoy-core` | Discord IPC client, presence model, profiles, game list. Pure Rust, no GUI. |
| `crates/decoy-app` | App logic shared by the tray daemon and the window. |
| `crates/decoy-daemon` | Linux only: the small process that sits in the tray. |
| `src-tauri` | The desktop shell (Tauri 2). |
| `src` | The window (Svelte). |

On Linux the tray icon and the Discord connection live in a small separate process, and the window is started only when you open it. In the tray that process uses about 3 MB of memory; the window adds about 200 MB while it is open and gives it all back when closed. On Windows and macOS the window's webview is likewise destroyed when you close it.

## How it talks to Discord

Decoy uses Discord's local IPC socket (a Unix socket, or a named pipe on Windows), the same documented way games report what you are playing. It never sees a password or a token. Game names and icons come from Discord's public API and CDN.

## Checks

```sh
cargo test --workspace          # unit and integration tests, with a fake Discord
cargo clippy --workspace --all-targets -- -D warnings
pnpm check                      # types of the window code
scripts/smoke.sh <path to the built decoy binary> smoke-out
```

`scripts/smoke.sh` starts the real app with a sample profile, waits for the window to render, presses a few controls and writes a report and a screenshot. The CI workflow runs all of the above on Linux, Windows and macOS.

## Releases

Pushing a tag like `v1.2.3` builds installers for every system and attaches them to a draft release. The files have fixed names without the version (`Decoy-windows-x64-setup.exe` and so on), so the download links in the README always point at the latest release. Starting the `release` workflow by hand only builds the installers and keeps them as workflow artifacts.

## Linux notes

- The tray icon needs a panel with StatusNotifierItem support, which KDE, Hyprland bars and most others have; GNOME needs the AppIndicator extension. Without a tray Decoy still works from its window.
- Decoy also works with Vesktop and other clients that provide the Discord IPC socket (arRPC).
- The window follows your monitor's refresh rate. WebKitGTK falls back to 60 fps when the graphics driver refuses its vertical-blank wait (NVIDIA's proprietary driver does), so Decoy paces frames itself in that case and uses the non-dmabuf renderer on NVIDIA. `DECOY_VBLANK=off` and `WEBKIT_DISABLE_DMABUF_RENDERER=0` switch these back.
