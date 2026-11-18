#!/usr/bin/env bash
# Smoke test: start the app, wait until its window reports that it rendered
# and that the commands answer, take a screenshot, check the report. Then start
# it once more, type into a field and close the window in the same instant: the
# edit must be on disk afterwards.
#
#   scripts/smoke.sh <path to the decoy binary> [output directory]
#
# CI runs this on Linux, Windows and macOS. The app must be built with the
# bundled frontend (cargo build --features tauri/custom-protocol, or cargo tauri build).
set -uo pipefail

bin="$1"
out="${2:-smoke}"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
os="$(uname -s)"

case "$os" in
  Linux) cfg="${XDG_CONFIG_HOME:-$HOME/.config}/dev.decoy.app" ;;
  Darwin) cfg="$HOME/Library/Application Support/dev.decoy.app" ;;
  *) cfg="$(cygpath -u "$APPDATA")/dev.decoy.app" ;;
esac
# Start with a filled profile so the screenshot shows the editor, not the first-run screen.
mkdir -p "$cfg"
seeded=0
[ -e "$cfg/profiles.json" ] || { cp "$here/smoke-profile.json" "$cfg/profiles.json"; seeded=1; }

report="$out/report.json"
rm -f "$report"
case "$os" in
  Linux | Darwin) export DECOY_SMOKE="$report" ;;
  *) export DECOY_SMOKE="$(cygpath -w "$report")" ;;
esac
export DECOY_SMOKE_HOLD="${DECOY_SMOKE_HOLD:-8}"
daemons_before="$(pgrep -x decoy-daemon 2>/dev/null | tr '\n' ' ')"

"$bin" > "$out/app.log" 2>&1 &
pid=$!
for _ in $(seq 1 110); do
  [ -s "$report" ] && break
  kill -0 "$pid" 2>/dev/null || break
  sleep 1
done
sleep 2

case "$os" in
  Linux)
    if [ -n "${WAYLAND_DISPLAY:-}" ] && command -v grim > /dev/null; then
      grim "$out/screen.png"
    elif command -v import > /dev/null; then
      import -window root "$out/screen.png"
    fi
    ;;
  Darwin) screencapture -x "$out/screen.png" ;;
  *)
    powershell -NoProfile -Command "
      Add-Type -AssemblyName System.Windows.Forms, System.Drawing
      \$b = [System.Windows.Forms.SystemInformation]::VirtualScreen
      \$bmp = New-Object System.Drawing.Bitmap \$b.Width, \$b.Height
      \$g = [System.Drawing.Graphics]::FromImage(\$bmp)
      \$g.CopyFromScreen(\$b.Left, \$b.Top, 0, 0, \$bmp.Size)
      \$bmp.Save('$(cygpath -w "$out/screen.png")')"
    ;;
esac

wait "$pid"
code=$?

# Only with the sample profile: this run changes the profile's display name.
saved=skipped
if [ "$seeded" = 1 ]; then
  unset DECOY_SMOKE
  DECOY_SMOKE_EDIT=1 "$bin" > "$out/close.log" 2>&1 &
  pid=$!
  for _ in $(seq 1 60); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 1
  done
  kill "$pid" 2>/dev/null
  wait "$pid" 2>/dev/null
  if grep -q 'typed right before closing' "$cfg/profiles.json"; then saved=yes; else saved=no; fi
fi

# Linux: the window started the tray daemon; stop the one this run started.
for d in $(pgrep -x decoy-daemon 2>/dev/null); do
  case " $daemons_before " in *" $d "*) ;; *) kill "$d" 2>/dev/null ;; esac
done

echo "exit code: $code"
echo "edit made right before closing was saved: $saved"
echo "report: $(cat "$report" 2>/dev/null || echo missing)"
[ -s "$out/app.log" ] && { echo "--- app log"; tail -n 30 "$out/app.log"; }
[ -s "$out/close.log" ] && { echo "--- close test log"; tail -n 15 "$out/close.log"; }
[ "$code" = 0 ] && [ "$saved" != no ] && grep -q '"ok":true' "$report" 2>/dev/null
