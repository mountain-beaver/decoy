# Decoy

Looks busy so you don't have to.

Custom Discord Rich Presence for every desktop. Pick any game Discord knows, write your own text, and Decoy shows it on your profile. It lives in the tray and stays out of the way.

Works on Windows, macOS and Linux.

## What it does

- **Any game, with its real name and icon.** Search Discord's own list of about 25,000 games. The small icon next to your name in voice channels comes from the game you pick, so it looks exactly like the real thing.
- **Or your own application.** Paste an application ID from the Discord Developer Portal to use your own name, icon and uploaded images.
- **Everything Rich Presence can show.** Playing, Listening to, Watching or Competing in. Two lines of text, party size, a timer (since you turned it on, time of day, or a countdown), large and small images, up to two buttons, links on text and images.
- **A live preview** of your profile card, the member list and the voice channel row, updated as you type.
- **A built-in guide.** Step by step: how to create your own application in the Discord Developer Portal, where each field shows up, and what to check when nothing appears.
- **Profiles.** Keep several presences and switch between them from the tray. Export a profile to a file and import it on another computer.
- **Starts with your computer** and reconnects by itself whenever Discord restarts.

## Install

Download the installer for your system from the [Releases](../../releases) page.

Discord has to be running on the same computer. On Linux, Decoy also works with Vesktop and other clients that provide the Discord IPC socket (arRPC).

## How it works

Decoy talks to the Discord client on your computer through Discord's local IPC socket, the same documented way games report what you are playing. It never asks for your password or token and does not touch your account.

Game names and icons are loaded from Discord's public API and CDN when you use the app. No artwork is stored in this repository.

## Build from source

You need Rust, Node.js with pnpm, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your system.

```sh
pnpm install
pnpm tauri build
```

On NixOS, run `nix develop` and build with `cargo tauri build` instead (the npm build of the Tauri CLI does not run there).

Project layout:

| Path | What it is |
|---|---|
| `crates/decoy-core` | Discord IPC client, presence model, profiles, game list. Pure Rust, no GUI. |
| `crates/decoy-app` | App logic shared by the tray daemon and the window. |
| `crates/decoy-daemon` | Linux only: the small process that sits in the tray. |
| `src-tauri` | The desktop shell (Tauri 2). |
| `src` | The window (Svelte). |

On Linux the tray icon and the Discord connection live in a small separate process, and the window is started only when you open it. This keeps memory use low while Decoy sits in the tray.

## Things to know

- On Linux the window follows your monitor's refresh rate. WebKitGTK falls back to 60 fps when the graphics driver refuses its vertical-blank wait (NVIDIA's proprietary driver does), so Decoy paces frames itself in that case and uses the non-dmabuf renderer on NVIDIA. `DECOY_VBLANK=off` and `WEBKIT_DISABLE_DMABUF_RENDERER=0` switch these back.

- Discord does not let you click your own buttons. Other people can.
- Discord accepts about five presence updates every 20 seconds, so fast edits are sent in batches.
- A custom display name is sent to Discord, but some clients ignore it and show the application's name.

## License

MIT. See [LICENSE](LICENSE).

Decoy is not affiliated with Discord or any game publisher. Game names and icons belong to their owners.
