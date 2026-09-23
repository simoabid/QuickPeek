# QuickPeek Project Context

## Project Goal
QuickPeek is a lightweight, responsive Linux space-to-preview file viewer that re-implements the fast preview user experience of the Windows application QuickLook. The initial v1 scope delivers instant previewing for common image formats (PNG, JPG, GIF, WebP, and SVG) when browsing files in Nautilus and Dolphin file managers under Wayland compositors (Hyprland and Niri) on Arch/CachyOS. For full lifecycle tracking, milestone targets, and phase status, see [PROGRESS.md](PROGRESS.md).

## Environment Facts

### 1a — Compilers and Toolchains
- `rustc --version`:
  ```
  rustc 1.98.1 (48a229cea 2026-09-01) (Arch Linux rust 1:1.98.1-1.1)
  ```
- `cargo --version`:
  ```
  cargo 1.98.1 (797e8a9bc 2026-08-05) (Arch Linux rust 1:1.98.1-1.1)
  ```
- `pkg-config --modversion gtk4`:
  ```
  4.22.5
  ```

### 1b — Compositor and Session State
- `hyprctl version`:
  ```
  HYPRLAND_INSTANCE_SIGNATURE not set! (is hyprland running?)
  ```
  (exit code: 1; Hyprland is installed but not active during this session)
- `echo $XDG_CURRENT_DESKTOP $XDG_SESSION_TYPE`:
  ```
  niri wayland
  ```
- `hyprctl activewindow -j`:
  ```
  HYPRLAND_INSTANCE_SIGNATURE not set! (is hyprland running?)
  ```
  (exit code: 1)
- Active window under running compositor (`niri msg -j focused-window`):
  ```json
  {"id":7,"title":"Welcome - QuickPeek - Visual Studio Code","app_id":"code","pid":6405,"workspace_id":3,"is_focused":true,"is_floating":false,"is_urgent":false,"layout":{"pos_in_scrolling_layout":[3,1],"tile_size":[1908.0,1026.0],"window_size":[1908,1026],"tile_pos_in_workspace_view":null,"window_offset_in_tile":[0.0,0.0]},"focus_timestamp":{"secs":6387,"nanos":372787301}}
  ```

### 1c — Niri Compositor Inspection
- `niri --version`:
  ```
  niri 26.04 (8ed0da4)
  ```
- `niri msg --help 2>&1 | head -30`:
  ```
  Communicate with the running niri instance

  Usage: niri msg [OPTIONS] <COMMAND>

  Commands:
    outputs           List connected outputs
    workspaces        List workspaces
    windows           List open windows
    layers            List open layer-shell surfaces
    keyboard-layouts  Get the configured keyboard layouts
    focused-output    Print information about the focused output
    focused-window    Print information about the focused window
    pick-window       Pick a window with the mouse and print information about it
    pick-color        Pick a color from the screen with the mouse
    action            Perform an action
    output            Change output configuration temporarily
    event-stream      Start continuously receiving events from the compositor
    version           Print the version of the running niri instance
    request-error     Request an error from the running niri instance
    overview-state    Print the overview state
    casts             List screencasts
    help              Print this message or the help of the given subcommand(s)

  Options:
    -j, --json  Format output as JSON
    -h, --help  Print help
  ```

### 1d — File Managers
- `nautilus --version`:
  ```
  ** Message: 03:11:28.229: Connecting to org.freedesktop.Tracker3.Miner.Files
  GNOME nautilus 50.3.1
  ```
- `dolphin --version`:
  ```
  dolphin 26.08.1
  ```

### 1e — Session D-Bus Inspection
- `busctl --user list | grep -Ei 'atspi|a11y|nautilus|dolphin|quickpeek'`:
  ```
  :1.107                                                                                                   17085 nautilus        seemoo :1.107        user@1000.service -       -
  org.a11y.Bus                                                                                              1003 at-spi-bus-laun seemoo :1.14         user@1000.service -       -
  org.freedesktop.FileManager1                                                                             17085 nautilus        seemoo :1.107        user@1000.service -       -
  org.freedesktop.a11y.Manager                                                                               946 niri            seemoo :1.11         user@1000.service -       -
  org.gnome.Nautilus                                                                                       17085 nautilus        seemoo :1.107        user@1000.service -       -
  ```

### 1f — AT-SPI Bus Address and Registration
- Verbatim prompt probe `gdbus call --session --dest org.a1ly.Bus --object-path /org/a11y/bus --method org.a11y.Bus.GetAddress`:
  ```
  Error: GDBus.Error:org.freedesktop.DBus.Error.ServiceUnknown: The name is not activatable
  ```
  (exit code: 1; expected failure due to verbatim prompt destination typo `org.a1ly.Bus` with digit `1`)
- Standard destination call `gdbus call --session --dest org.a11y.Bus --object-path /org/a11y/bus --method org.a11y.Bus.GetAddress`:
  ```
  ('unix:path=/run/user/1000/at-spi/bus_0',)
  ```
  (exit code: 0)
- `busctl --address=unix:path=/run/user/1000/at-spi/bus_0 list 2>&1 | head -40`:
  ```
  NAME                      PID PROCESS         USER   CONNECTION UNIT              SESSION DESCRIPTION
  :1.0                      999 nm-applet       seemoo :1.0       user@1000.service -       -
  :1.1                     1060 xdg-desktop-por seemoo :1.1       user@1000.service -       -
  :1.15                   17085 nautilus        seemoo :1.15      user@1000.service -       -
  :1.16                   17994 busctl          seemoo :1.16      user@1000.service -       -
  :1.2                     1064 qs              seemoo :1.2       user@1000.service -       -
  :1.3                     1111 at-spi2-registr seemoo :1.3       user@1000.service -       -
  :1.4                      987 blueman-applet  seemoo :1.4       user@1000.service -       -
  :1.6                     1187 blueman-tray    seemoo :1.6       user@1000.service -       -
  org.a11y.atspi.Registry  1111 at-spi2-registr seemoo :1.3       user@1000.service -       -
  org.freedesktop.DBus     1003 at-spi-bus-laun seemoo -          user@1000.service -       -
  ```
- **Registration Analysis**:
  - `org.gnome.Nautilus`: Registered (`:1.15`, PID 17085) automatically via GTK4 accessibility infrastructure.
  - `org.kde.dolphin`: Not registered by default in a bare compositor session. Probing confirmed Dolphin registers (`:1.19`, PID 18448) when launched with `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1`.

### 1g — Screenshot Tooling
- `grim --version`:
  ```
  grim: invalid option -- '-'
  ```
  (exit code: 1; `grim` lacks `--version` flag; `pacman -Q grim` reports version `grim 1.5.0-2.1`)
- `wl-paste --version`:
  ```
  wl-clipboard 2.3.0
  ```
  (exit code: 0)

### 1h — Name-Collision Checks (quickpeek)
- **GitHub Search** (`https://api.github.com/search/repositories?q=quickpeek`):
  36 total results. All are unrelated utilities (e.g. `AndrewRadev/quickpeek.vim` for Vim quickfix, `dodo-reach/QuickPeek` for macOS menu bar metrics). Target repo `simoabid/QuickPeek` is available and dedicated to this project.
- **AUR** (`https://aur.archlinux.org/rpc/v5/search/quickpeek`):
  0 results (`resultcount: 0`). No package named `quickpeek` exists in AUR.
- **crates.io** (`https://crates.io/api/v1/crates/quickpeek`):
  404 Not Found (`crate quickpeek does not exist`). The name is completely unclaimed on crates.io.

## Architecture & Implementation State
- `[PROVEN - Phase 1]` Single native GTK4 binary written in Rust using `gtk4-rs` (`gtk4 = "0.11.5"`).
- `[PROVEN - Phase 2]` D-Bus single-instance daemon owning `org.quickpeek.QuickPeek` on the session bus with pre-GTK role split.
  - **Service**: `org.quickpeek.QuickPeek`
  - **Object Path**: `/org/quickpeek/QuickPeek`
  - **Interface**: `org.quickpeek.QuickPeek`
  - **Methods**:
    - `ShowFile(in s path, out b ok, out s message)`: Displays the image at `path`. Swaps the image if the window is already open. Returns `ok=true, message=""` on success, or `ok=false, message="<error>"` on missing file or decode failure.
    - `Quit()`: Closes window, releases bus name, cleanly terminates event loop and daemon process with exit code 0.
  - **Role Decision Semantics**:
    - Synchronous `RequestName` call executed via `gtk4::gio` on `DBusConnection` before any `gtk::init()` call.
    - Reply 1 (`PRIMARY_OWNER`): Bus name acquired; process assumes **Daemon** role, initializes GTK4, registers `/org/quickpeek/QuickPeek`, displays preview window, and runs GLib main event loop.
    - Reply 3 (`EXISTS`): Bus name already owned; process assumes **Client** role, never initializes GTK4, invokes D-Bus method synchronously with a 2000 ms timeout, prints latency (if ShowFile), and exits.
    - Other replies: Emits error to stderr and exits 1.
  - **Daemon Lifecycle Decision**:
    - Daemon runs in the foreground in v1/v2 (no fork, no daemonize); systemd user service handles background lifecycle in Phase 7.
- `[PROVEN - Phase 3]` Super+Space Toggle Keybinding & Compositor Integration.
  - **D-Bus `Toggle()` Method**:
    - Added to `org.quickpeek.QuickPeek` interface: `Toggle() -> (b ok, s message)`.
    - Window open: destroys window, logs `TOGGLE_CLOSED` + `TOGGLE_MS <ms>`, returns `(true, "")`.
    - Window closed + `current_selection` exists: displays preview window, logs `TOGGLE_OPENED` + `WINDOW_OPENED` + `WARM_MS <ms>` + `TOGGLE_MS <ms>`, returns `(true, "")`.
    - Window closed + `current_selection` is `None`: returns `(false, "no file to preview")`.
  - **CLI Contract (Amended in Phase 4)**:

    | Invocation | Daemon State | Behavior |
    | :--- | :--- | :--- |
    | `quickpeek <path>` | any | ShowFile; persists selection (unchanged) |
    | `quickpeek` (no args) | running | Toggle(): if open, close; if closed, open (1) live Dolphin AT-SPI selection, (2) in-memory selection, (3) persisted selection |
    | `quickpeek` (no args) | NOT running | start daemon + OPEN (1) live Dolphin AT-SPI selection, (2) persisted selection; if none, silent start (service mode) |
    | `quickpeek --service` | NOT running | start daemon only; NEVER opens a window (autostart) |
    | `quickpeek --service` | running | exit 0 silently (idempotent — autostart never errs) |

  - **State Persistence ($XDG_STATE_HOME/quickpeek/last_selection)**:
    - Path persisted atomically (temp file + rename, mode 0700) on every successful selection set (`ShowFile`, toggle-open).
    - Restored on daemon startup (`SELECTION_RESTORED <path>` / `SELECTION_NONE`).
    - Stale file check on open: if file no longer exists, logs `Error: last previewed file no longer exists: <path>`, clears selection, and stays alive without opening window.

  - **Canonical Instrumentation Grammar**:
    ```
    "ROLE DAEMON" | "ROLE CLIENT" | "WINDOW_OPENED" | "IMAGE_SWAPPED" | "WINDOW_CLOSED" |
    "DAEMON_QUIT" | "TOGGLE_OPENED" | "TOGGLE_CLOSED" | "SELECTION_RESTORED <path>" |
    "SELECTION_NONE" | "SELECTION_LIVE <path>" | "SELECTION_FALLBACK <memory|persisted|none>" |
    "ATSPI_MS <n>" | "ATSPI_UNAVAILABLE <reason>" | "MAP_MS <n>" | "CALL_MS <n>" |
    "WARM_MS <n>" | "TOGGLE_MS <n>"
    ```
    - `SELECTION_RESTORED <path>`: emitted once at daemon start when persisted selection exists.
    - `SELECTION_NONE`: emitted once at daemon start when no persisted selection exists.
    - `SELECTION_LIVE <path>`: emitted when an open trigger resolves a selected file from the active Dolphin window via AT-SPI.
    - `SELECTION_FALLBACK <memory|persisted|none>`: emitted when AT-SPI resolution is unavailable/inapplicable, indicating fallback target.
    - `ATSPI_MS <n>`: measured duration in milliseconds of the AT-SPI discovery attempt.
    - `ATSPI_UNAVAILABLE <reason>`: emitted whenever AT-SPI discovery fails or is skipped (fail-open loud).
    - `WARM_MS <n>`: image display completion (emitted on swap, toggle-open, or initial open).
    - `MAP_MS <n>`: additionally emitted on each window map event (canonical real-bus baseline: **58 ms**).
    - `TOGGLE_MS <n>`: measured from D-Bus handler entry to toggle action completion (both directions).
  - **Compositor Keybinding (Niri & Hyprland)**:
    - **Niri (`~/.config/niri/config.kdl`)**: `Mod+Space hotkey-overlay-title="QuickPeek Preview" { spawn "quickpeek"; }`. Takes over `Mod+Space` from duplicate DMS spotlight launcher (DMS spotlight remains 100% active on `Mod+D`). Revert path: uncomment marked block in `config.kdl` or restore `~/.config/niri/config.kdl.quickpeek-backup`.
    - **Hyprland (`~/.config/hypr/hyprland.conf`)**: `bind = SUPER, space, exec, quickpeek` (documented in `docs/keybindings.md`; live verification pending human login).
    - **Compositor-Global Dispatch**: Shortcut triggers regardless of focused window, unlike `Escape` which requires preview window focus.
    - **Bare-Space Stretch Note**: Unmodified `Space` preview is explicitly a Phase 8 stretch experiment exploring window-context filtering and synthetic key re-injection.
- `[PROVEN - Phase 4]` Active file selection discovery via AT-SPI2 for Dolphin:
  - **Transport**: Native `gtk4::gio` raw D-Bus calls only (`gio::DBusConnection::for_address_sync` with `AUTHENTICATION_CLIENT | MESSAGE_BUS_CONNECTION`). Zero new crates (`Cargo.lock` diff = 0).
  - **Dynamic Bus Discovery**: Queries `org.a11y.Bus` -> `/org/a11y/bus` -> `GetAddress` on the daemon's own session connection dynamically.
  - **Fail-Open Loud & Safe Calls**: Every call specifies `DBusCallFlags::NO_AUTO_START` preventing systemd auto-activation hangs. Failures emit `ATSPI_UNAVAILABLE <reason>` and fall back within $\le 5\text{ ms}$.
  - **Active Window Filter**: Locates Dolphin among desktop children; inspects window children with `ROLE_FRAME` (23) and `ATSPI_STATE_ACTIVE` bit 1 (mask `1 << 1` on states `[0]`).
  - **Pruned Tree Walk**: Prunes menu bars, toolbars, status bars, and buttons to bound traversal to <30 nodes, running in 16.6 ms (well below 150 ms hard deadline).
  - **Path Derivation**: Traverses `ROLE_LIST` / `ROLE_TREE_TABLE` to extract primary selected item text. Derives directory path from `KUrlNavigator` combo box / breadcrumb children, falling back to window title caption (`— Dolphin`).
  - Nautilus AT-SPI selection discovery planned for Phase 5.

## Conventions
- **Git Branch**: `main` as the default development and production branch.
- **Conventional Commits**: All commit messages adhere strictly to `feat:`, `fix:`, `docs:`, `chore:`, `test:`, or `refactor:`.
- **Evidence or it Didn't Happen**: Every technical decision, bug claim, or verification must be substantiated with exact terminal commands, exit codes, and verbatim outputs.
- **REPORTS Convention (Permanent)**: Create gitignored `REPORTS/` (listed in `.gitignore`); every phase's final report is written as `REPORTS/PHASE_X_REPORT.md` AND pasted in full in chat reply.

## Crate Layout
- **Root Product Crate (`quickpeek`)**:
  - `Cargo.toml`: defines root `quickpeek` binary crate with `gtk4 = "0.11"` dependency (zero extra crates).
  - `Cargo.lock`: pinned and committed dependencies (64 packages, 0 diff in Phase 2, Phase 3, and Phase 4).
  - `.cargo/config.toml`: repo-level target linker fix (`[target.x86_64-unknown-linux-gnu] linker = "gcc"`).
  - `src/main.rs`: product entrypoint, pre-GTK role decision, daemon GTK loop, client fast path.
  - `src/dbus.rs`: D-Bus constants, XML introspection, role types, `RequestName`, `call_show_file`, `register_server`.
  - `src/window.rs`: `WindowManager`, aspect-fit math, texture loading/swapping, key handling, and window lifecycle.
  - `src/atspi.rs`: AT-SPI D-Bus client, role pruning, active window detection, selection extraction, and unit tests.
  - `tests/e2e_dbus.sh`: hermetic e2e test harness running under `dbus-run-session` (16 automated checks).
  - `tests/manual_atspi.sh`: live Dolphin AT-SPI verification script.
  - `docs/selection.md`: Dolphin environment setup (`QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1`), AT-SPI architecture, and resolution hierarchy.
  - `tests/fixtures/sample.png`: 64x64 RGBA test image fixture.
  - `tests/fixtures/sample2.png`: 128x96 RGBA test image fixture for image swap tests.
  - `tests/fixtures/notimage.txt`: non-image text fixture for decode rejection tests.
- **Probe Crate (`probe/`)**:
  - Gitignored throwaway instrumentation crate for toolchain verification and asset generation.

## Integration Points
- **Hyprland Binds + `hyprctl`**: Global shortcut `bind = , space, exec, quickpeek --toggle` and active window inspection via `hyprctl activewindow -j`.
- **Niri Binds + `niri msg`**: Global shortcut `binds { Mod+Space { spawn "quickpeek" "--toggle"; } }` and window inspection via `niri msg -j focused-window`.
- **Nautilus AT-SPI**: Interrogation of `org.gnome.Nautilus` on the AT-SPI bus to extract URI / path from the selected file item in GTK icon/list view.
- **Dolphin AT-SPI**: Interrogation of `org.kde.dolphin` on the AT-SPI bus to extract selected file path from Dolphin's item view (requires `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1`).
- **Session D-Bus**: Communication channel between external shortcut invocations and the single-instance QuickPeek preview daemon.

## Risks
1. **AT-SPI Selection Fragility**:
   - *Risk*: File managers may change internal accessible tree hierarchies between minor releases or under different view modes (icon view, list view, compact view), causing selected item extraction to fail.
   - *Probe*: Build a dedicated test harness probing AT-SPI object trees for both Nautilus and Dolphin across all view layouts and rapid selection changes.
2. **Niri-vs-Hyprland Bind Differences**:
   - *Risk*: Compositor shortcut dispatch behavior and window management semantics (e.g. focused window reporting, surface layering) differ significantly between Hyprland and Niri.
   - *Probe*: Create test script verifying compositor active-window query round-trip latency and key dispatch reliability under both Hyprland and Niri.
3. **Wayland Window-Positioning Limits**:
   - *Risk*: Pure Wayland security models prevent arbitrary client-side window positioning over existing foreign application windows without compositor-specific protocol support (e.g. layer-shell or xdg-foreign).
   - *Probe*: Implement a minimal GTK4 popup probe testing window center-screen placement vs layer-shell surface anchoring over file manager surfaces.
