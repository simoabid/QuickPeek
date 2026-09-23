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
    - Reply 3 (`EXISTS`): Bus name already owned; process assumes **Client** role, never initializes GTK4, invokes `ShowFile(path)` synchronously with a 2000 ms timeout, prints latency, and exits.
    - Other replies: Emits error to stderr and exits 1.
  - **Instrumentation Grammar**:
    - `MAP_MS <ms>`: Daemon, printed once at first window map (`connect_map`).
    - `CALL_MS <ms>`: Client, printed per invocation measuring synchronous D-Bus call round-trip.
    - `WARM_MS <ms>`: Daemon, printed per image swap while the window is already mapped.
  - **Daemon Lifecycle Decision**:
    - Daemon runs in the foreground in v1/v2 (no fork, no daemonize); systemd user service handles background lifecycle in Phase 7.
- `[PLANNED]` Compositor-configured keybindings (`Space` to toggle preview, `Escape` to close) bound in Hyprland (`hyprland.conf`) and Niri (`config.kdl`) invoking `quickpeek` CLI / D-Bus method.
- `[PLANNED]` Active file selection discovery via the AT-SPI2 accessibility D-Bus (`unix:path=/run/user/$UID/at-spi/bus_0`), querying the focused file manager view's selected accessible items.

## Conventions
- **Git Branch**: `main` as the default development and production branch.
- **Conventional Commits**: All commit messages adhere strictly to `feat:`, `fix:`, `docs:`, `chore:`, `test:`, or `refactor:`.
- **Evidence or it Didn't Happen**: Every technical decision, bug claim, or verification must be substantiated with exact terminal commands, exit codes, and verbatim outputs.
- **REPORTS Convention (Permanent)**: Create gitignored `REPORTS/` (listed in `.gitignore`); every phase's final report is written as `REPORTS/PHASE_X_REPORT.md` AND pasted in full in chat reply.

## Crate Layout
- **Root Product Crate (`quickpeek`)**:
  - `Cargo.toml`: defines root `quickpeek` binary crate with `gtk4 = "0.11"` dependency (zero extra crates).
  - `Cargo.lock`: pinned and committed dependencies (64 packages, 0 diff in Phase 2).
  - `.cargo/config.toml`: repo-level target linker fix (`[target.x86_64-unknown-linux-gnu] linker = "gcc"`).
  - `src/main.rs`: product entrypoint, pre-GTK role decision, daemon GTK loop, client fast path.
  - `src/dbus.rs`: D-Bus constants, XML introspection, role types, `RequestName`, `call_show_file`, `register_server`.
  - `src/window.rs`: `WindowManager`, aspect-fit math, texture loading/swapping, key handling, and window lifecycle.
  - `tests/e2e_dbus.sh`: hermetic e2e test harness running under `dbus-run-session`.
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
