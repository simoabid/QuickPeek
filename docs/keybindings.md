# QuickPeek Keybinding Setup Guide (Niri & Hyprland)

QuickPeek brings the Windows "QuickLook" space-to-preview user experience to Linux Wayland compositors. On Wayland, individual client applications cannot install global keyboard hooks; the compositor directly owns and dispatches all global keybindings.

---

## 1. Installation into Session PATH

Because compositor shortcuts invoke the bare command name `quickpeek`, the compiled binary must exist in a directory on the session's `$PATH` (such as `~/.local/bin`):

```bash
install -Dm755 target/release/quickpeek ~/.local/bin/quickpeek
```

> [!IMPORTANT]
> The command above installs a **copy** of the binary. After every `cargo build --release`, you **must re-run** the install command to update `~/.local/bin/quickpeek`.
>
> **Symlink Alternative (Developer Mode):**
> If you prefer automatic updates upon building:
> ```bash
> ln -sf "$PWD/target/release/quickpeek" ~/.local/bin/quickpeek
> ```

---

## 2. Autostart Configuration

To ensure the daemon is always primed and responsive from login, configure compositor autostart:

### Niri (`~/.config/niri/config.kdl`)
Add to the top-level startup section:
```kdl
spawn-at-startup "quickpeek" "--service"
```

### Hyprland (`~/.config/hypr/hyprland.conf`)
Add to `~/.config/hypr/hyprland.conf`:
```ini
exec-once = quickpeek --service
```

The `--service` flag is idempotent: if the daemon is already running, it exits `0` silently.

---

## 3. Niri Keybinding (`~/.config/niri/config.kdl`)

### Active Snippet
In the `binds { ... }` block of `~/.config/niri/config.kdl`:

```kdl
Mod+Space hotkey-overlay-title="QuickPeek Preview" { spawn "quickpeek"; }
```

### The DankMaterialShell (DMS) Takeover & Revert Path
- **DMS Spotlight Launcher on `Mod+D`**: Prior to Phase 3, DMS registered duplicate launcher bindings for both `Mod+Space` and `Mod+D`. In Phase 3, `Mod+Space` was reassigned to `quickpeek`. The DMS spotlight launcher remains 100% active on `Mod+D` (`Mod+D hotkey-overlay-title="Application Launcher" { spawn "dms" "ipc" "call" "spotlight" "toggle"; }`).
- **Revert Instructions**:
  1. Open `~/.config/niri/config.kdl`.
  2. Locate the QuickPeek section around line 382.
  3. Delete the `Mod+Space` QuickPeek line and uncomment the original 3-line `Mod+Space` DMS block:
     ```kdl
     Mod+Space hotkey-overlay-title="Application Launcher" {
         spawn "dms" "ipc" "call" "spotlight" "toggle";
     }
     ```
  4. Alternatively, restore the backup copy created at `~/.config/niri/config.kdl.quickpeek-backup`.

---

## 4. Hyprland Keybinding (`~/.config/hypr/hyprland.conf`)

*(Docs-only: compositor inactive in current session. Live verification pending human login into Hyprland).*

Add to `~/.config/hypr/hyprland.conf`:

```ini
bind = SUPER, space, exec, quickpeek
```

---

## 5. How Keybindings & Persistence Work

- **Cold-Press & Persistence Semantics**:
  - `Mod+Space` re-opens your last previewed file even after reboot; live file-manager selection = Phase 4.
  - QuickPeek persists your last previewed file path atomically to `$XDG_STATE_HOME/quickpeek/last_selection` (`~/.local/state/quickpeek/last_selection`).
  - If the daemon is not running when you press `Mod+Space`, it cold-starts and immediately opens the persisted selection without requiring a second keypress.
  - If no persisted selection exists or the file was deleted, it starts silently in service mode.

- **Compositor-Global Dispatch**:
  - `Mod+Space` (or `Super+Space`) is intercepted and handled by the compositor directly.
  - The shortcut triggers regardless of which window currently has keyboard focus (e.g. Dolphin, Nautilus, a terminal, or empty workspace).
  - This contrasts with `Escape`, which is a client-side GTK window key handler and requires the QuickPeek preview window to be focused to close it.

- **Server-Side Debounce**:
  - `Toggle()` requests received within 120 ms of window open are debounced to prevent accidental rapid double-presses from instantly closing a newly opened preview.

- **Stopping the Daemon**:
  - To stop the running QuickPeek background daemon manually:
    ```bash
    pkill -x quickpeek
    ```
  - Or via D-Bus directly:
    ```bash
    gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.Quit
    ```

- **Bare `Space` Key (Unmodified Space)**:
  - Binding bare `Space` globally at the compositor level breaks normal typing in text fields, editors, and terminal emulators across the desktop.
  - Unmodified `Space` preview is explicitly a Phase 8 stretch experiment exploring conditional window-context filtering and synthetic key re-injection.
  - For v1 core reliability, `Mod+Space` / `Super+Space` is the authoritative, conflict-free shortcut.

---

## 6. Troubleshooting Box

If `Mod+Space` appears to do nothing, run these four checks in order:

```bash
# 1. Verify quickpeek is in PATH and resolves
command -v quickpeek

# 2. Check if the daemon process is running
pgrep -x quickpeek

# 3. Check if the D-Bus service is registered on the user session bus
busctl --user list | rg quickpeek

# 4. Check user journal logs for spawn errors or daemon messages
journalctl --user -b | rg -i quickpeek
```
