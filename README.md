<p align="center">
  <img src="./docs/logo.svg" width="120" alt="gamacros logo" />
</p>

## Gamacros

Highly effective conversion of a gamepad into a macropad for applications. Gamacros listens for controller input and maps it to keyboard shortcuts based on the currently active application.

### Highlights

- **Per‑app mappings**: Switches rules automatically when the frontmost app changes.
- **YAML profiles**: Human‑readable with versioned schema.
- **Button chords and D‑pad**: Bind single buttons, multi‑button chords, or a D‑pad directional map.
- **Device remapping**: Per‑VID/PID logical remaps (e.g., Nintendo A/B, X/Y swap).
- **Haptics**: Optional short rumble on action, when supported.

## How it works

1. `gamacrosd` (daemon) starts an SDL2 runtime to enumerate controllers and emit button events.
2. A small Cocoa listener publishes the bundle identifier of the current frontmost app.
3. On button press/release, the active app’s rules are evaluated. Matching rules generate actions.
4. Actions send key events; optional rumble is dispatched if supported.

## Installation

### Script

You can use a simple [script](https://github.com/mishamyrt/gamacros/blob/refs/heads/main/scripts/install.sh) to install gamacros.
It will download the latest version of the binary and install it to the system.

```bash
# Install latest version
curl -sSfL https://raw.githubusercontent.com/mishamyrt/gamacros/refs/heads/main/scripts/install.sh | bash
```

## Usage

- Put a `.gc_profile.yaml` in the `$HOME` directory.
- Run the daemon in foreground mode (`gamacrosd run`) and grant accessibility permission when prompted.
- Switch applications; rules for the frontmost app will apply automatically.

Tip: To find a bundle id and controller vid/pid, you can use `run` in verbose mode (`-v`).

## Profile

The daemon searches for the configuration in the following files (in this order):

- `$HOME/Library/Application Support/gamacros/gc_profile.yaml`
- `$HOME/.gc_profile.yaml`

Custom profile path can be set with the `--profile` command line argument.

### Schema (version 1)

- **version**: profile schema version (must be `1`).
- **controllers**: optional list of device remaps by USB `vid`/`pid` with `remap` map.
- **shell**: optional shell path for shell actions (e.g., `/bin/zsh`).
- **blacklist**: bundle IDs to ignore when matching apps.
- **groups**: named lists of bundle IDs for reuse in selectors.
- **rules**: mapping of selectors → app rules. Special key `common` applies to all.
  - App rules:
    - `buttons`: `<chord>` → `{ vibrate?, keystroke? | macros? | shell? }`
    - `sticks`: `left|right` → `{ mode: arrows|mouse_move|scroll|volume|brightness, ... }`

### Examples

Minimal profile with per‑app rules via selectors and a device remap:

```yaml
version: 1

controllers:
  - vid: 0x57e
    pid: 0x2009
    remap:
      a: b
      b: a

shell: /bin/zsh
blacklist:
  - Hades 2

groups:
  ide:
    - com.microsoft.VSCode
    - com.todesktop.230313mzl4w4u92 # Cursor
  browser:
    - com.google.Chrome
    - com.apple.Safari

rules:
  common:
    buttons:
      l1:
        vibrate: 100
        keystroke: rcmd # Local voice recognition hold-to-talk
      a:
        keystroke: enter

  $ide | $browser:
    buttons:
      y:
        shell: echo "$"
      select:
        keystroke: cmd+dot
      start:
        keystroke: cmd+i
      dpad_up:
        keystroke: arrow_up
      dpad_left:
        keystroke: arrow_left
      dpad_right:
        keystroke: arrow_right
      dpad_down:
        keystroke: arrow_down
      l2+r2:
        vibrate: 100
        macros: [cmd+a, backspace]
    sticks:
      right:
        mode: mouse_move
        max_speed_px_s: 1600
```

#### Buttons and chords

- Use logical button names and join with `+` for chords (e.g., `l2+r2`, `lb`, `a`).
- D‑pad directions are `dpad_up`, `dpad_down`, `dpad_left`, `dpad_right`.

#### Key combos (quick reference)

Examples: `cmd+shift+l`, `option+space`, `enter`, `backspace`, `arrow_up`.

## Permissions

To send key events on macOS, the process must be allowed under System Settings → Privacy & Security → Accessibility. The first run may prompt for permission; otherwise add the binary manually.

## Roadmap

- Button hold semantics.
- Gyro/accelerometer support.
- UI for editing profiles.

## License

MIT License. See `LICENSE`.
