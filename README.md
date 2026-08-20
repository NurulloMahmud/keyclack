# keyclack

Plays a mechanical-keyboard click sound every time you press a key, in any
application, so a laptop keyboard sounds like a mechanical one. macOS only,
run from source — no `.app` bundle, installer, or code signing.

## Requirements

- macOS (Apple Silicon or Intel)
- [Rust](https://rustup.rs) stable, 1.75 or newer

If you don't have Rust yet:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Quick start

```bash
git clone https://github.com/NurulloMahmud/keyclack.git
cd keyclack
cargo run --release
```

The window opens even if nothing plays yet — see the permission step below.
On the very first run, macOS will pop up its own "keyclack" (really your
terminal, see below) Accessibility prompt; grant it and restart the app.

To just build a binary without running it:

```bash
cargo build --release
# binary is at target/release/keyclack
```

## Accessibility permission

`keyclack` listens for key presses system-wide via a macOS Accessibility
event tap. macOS will not deliver key events to it until it is trusted.

**If you run it with `cargo run` from a terminal:** macOS attributes
Accessibility trust to the app that launched the process — your terminal
app, not `keyclack` itself, because the binary is unsigned and not bundled.
Grant access to your terminal application (Terminal.app, iTerm, etc.), not
to `keyclack`:

1. Open **System Settings → Privacy & Security → Accessibility** (the app
   also has an "Open System Settings" button in its permission banner).
2. Enable the toggle next to your terminal application.
3. Restart `keyclack` (or just restart the run — live re-attachment after
   granting permission is not supported in v1).

If the toggle isn't there yet, run `keyclack` once first — it will prompt
the system to add an entry for the terminal, then you can enable it.

## Adding a sound pack

Create a new directory under `assets/packs/`:

```
assets/packs/my-pack/
├── pack.json
└── press.wav
```

`pack.json`:

```json
{
  "id": "my-pack",
  "name": "My Pack",
  "file": "press.wav"
}
```

Requirements for `press.wav`:

- 16-bit integer PCM (mono or stereo; stereo is averaged down to mono).
- Any sample rate — it is resampled once at load time to match your audio
  device.
- Keep it under **300 ms**. Longer samples still play, but overlapping
  clicks during fast typing get muddy since v1 does not truncate long
  samples.

`keyclack` picks up new pack directories automatically the next time it
starts; malformed or missing packs are skipped with a warning in the log
rather than preventing startup.

## Global mute hotkey

**Control + Option + M** toggles mute from anywhere, even while the window
is unfocused or minimized.

## Start on login

Check "Start on login" in the app to install a LaunchAgent
(`~/Library/LaunchAgents/com.keyclack.agent.plist`) that launches the
current build of `keyclack` at login. If you move or rebuild the binary at
a different path, toggle the checkbox off and back on to repoint it.
