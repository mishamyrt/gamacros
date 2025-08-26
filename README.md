<p align="center">
  <img src="docs/logo.svg" width="120" alt="gamacros logo" />
</p>

## gamacros

Highly effective conversion of a gamepad into a macropad for applications. gamacros listens for controller input and maps it to keyboard shortcuts based on the currently active application.

### Highlights
- **Per‑app mappings**: Switches rules automatically when the frontmost app changes (via `NSWorkspace`).
- **YAML profiles**: Human‑readable `profile.yaml` with versioned schema.
- **Button chords and D‑pad**: Bind single buttons, multi‑button chords, or a D‑pad directional map.
- **Device remapping**: Per‑VID/PID logical remaps (e.g., Nintendo A/B, X/Y swap).
- **Haptics**: Optional short rumble on action, when supported.
- **Local, fast, offline**: No network or cloud dependencies.

### Status
- Platform: **macOS** (the activity monitor uses Cocoa APIs). Other platforms may work for controller input but are not supported yet.
- Rust: requires toolchain matching `rust-version` in workspace (`1.80`).

## How it works
1. `gamacrosd` starts an SDL2 runtime to enumerate controllers and emit button events.
2. A small Cocoa listener publishes the bundle identifier of the current frontmost app.
3. On button press/release, the active app’s rules are evaluated. Matching rules generate actions.
4. Actions send key events via `Enigo`; optional rumble is dispatched if supported.

## Architecture
- `crates/gamacrosd`: The daemon wiring everything together.
- `crates/gamacros-controller`: Device discovery, events, rumble (SDL2 backend).
- `crates/gamacros-activity`: macOS `NSWorkspace` listener for app focus changes.
- `crates/gamacros-keypress`: Parsing and performing key combos (Enigo).
- `crates/gamacros-profile`: Profile parser and runtime matcher.

## Profile
Place a `profile.yaml` in the working directory of `gamacrosd`. The daemon looks up `./profile.yaml` on startup (no hot‑reload yet).

### Schema (version 1)
- **version**: profile schema version (currently `1`).
- **gamepads**: optional list of device remaps by USB `vid`/`pid`.
  - `mapping`: logical button → logical button.
- **apps**: mapping of macOS bundle id → list of rules.
  - `trigger`: either a button chord string (e.g. `lt+rt`) or the literal `dpad`.
  - `action`: key combo string (for chords) or per‑direction map (for `dpad`).
  - `vibrate` (optional): milliseconds of rumble on action.
  - `when` (optional): `pressed` (default) or `released`.

### Example
```yaml
version: 1
gamepads:
  # Switch Pro Controller remap
  - vid: 0x57e
    pid: 0x2009
    mapping:
      a: b
      b: a
      x: y
      y: x

apps:
  com.todesktop.230313mzl4w4u92: # Cursor
    - trigger: lb
      action: option+space
    - trigger: rb
      action: cmd+z
    - trigger: lt+rt
      action: cmd+shift+l
      vibrate: 100
    - trigger: y
      action: cmd+z
    - trigger: b
      action: enter
    - trigger: a
      action: escape
    - trigger: select
      action: cmd+p
    - trigger: start
      action: cmd+shift+l
    - trigger: dpad
      action:
        down: arrow_down
        up: arrow_up
        left: arrow_left
        right: arrow_right
```

### Triggers
- **Chord**: `a`, `b`, `x`, `y`, `lb`, `rb`, `lt`, `rt`, `start`, `select`/`back`, `guide`/`home`, `ls`/`left_stick`, `rs`/`right_stick`, `up`, `down`, `left`, `right`.
  - Chords are `+`‑separated (e.g., `lt+rt`). Chord rules fire when all buttons in the set are held during the specified phase.
- **D‑pad**: `trigger: dpad` with `action.up/down/left/right` mapping.
- **Phase**: `when: pressed` (default) fires on press; `when: released` fires on release.

### Actions (key combos)
Key combo strings parse into modifiers and keys. Supported:
- **Modifiers**: `ctrl`, `cmd`/`meta`/`command`/`super`, `alt`/`option`, `shift`.
- **Navigation**: `arrow_up`, `arrow_down`, `arrow_left`, `arrow_right`, `home`, `end`, `page_up`, `page_down`.
- **Editing**: `tab`, `space`/`spacebar`, `enter`/`return`, `delete`, `backspace`, `escape`/`esc`.
- **System**: `volume_up`, `volume_down`, `volume_mute`, `brightness_up`, `brightness_down`, `illumination_up`, `illumination_down`.
- **Function keys**: `f1` … `f20`.
- **Single characters**: any single printable character (e.g., `a`).

Examples: `cmd+shift+l`, `ctrl+alt+delete`, `option+space`, `f13`.

### Device remapping
Use `gamepads` to swap physical‑to‑logical buttons for specific USB devices. `vid`/`pid` accept hex (e.g., `0x57e`) or decimal.

## Permissions
To send key events on macOS, the process must be allowed under System Settings → Privacy & Security → Accessibility. The first run may prompt for permission; otherwise add the binary manually.

## Usage
- Put a `profile.yaml` next to the `gamacrosd` working directory.
- Run the daemon and grant accessibility permission when prompted.
- Switch applications; rules for the frontmost app will apply automatically.

Tip: To find a bundle id, you can use AppleScript (e.g., `id of app "AppName"`).

## Installation
TODO: add installation and build instructions (Homebrew/cargo, codesigning, launch agent, etc.).

## Roadmap
- Hot‑reload `profile.yaml` on changes.
- Per‑app defaults and wildcards.
- UI for editing profiles.
- Windows/Linux support for activity monitoring and key synthesis.
- Advanced inputs (sticks/axes) and tap/hold semantics.

## License
MIT License. See `LICENSE`.

## Acknowledgements
- SDL2 for controller input and haptics.
- Enigo for cross‑platform key synthesis.
- Cocoa/Objective‑C runtime for macOS app activity.
