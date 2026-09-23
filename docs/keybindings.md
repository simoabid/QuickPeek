# QuickPeek Keybinding Setup Guide (Niri & Hyprland)

QuickPeek brings the Windows "QuickLook" space-to-preview user experience to Linux Wayland compositors. On Wayland, individual client applications cannot install global keyboard hooks; the compositor directly owns and dispatches all global keybindings.

---

## 1. Niri Configuration (`~/.config/niri/config.kdl`)

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

## 2. Hyprland Configuration (`~/.config/hypr/hyprland.conf`)

*(Docs-only: compositor inactive in current session. Live verification pending human login into Hyprland).*

### Snippet
Add to `~/.config/hypr/hyprland.conf`:

```ini
bind = SUPER, space, exec, quickpeek
```

---

## 3. How Keybindings Work in QuickPeek

- **Compositor-Global Dispatch**:
  - `Mod+Space` (or `Super+Space`) is intercepted and handled by the compositor directly.
  - The shortcut triggers regardless of which window currently has keyboard focus (e.g. Dolphin, Nautilus, a terminal, or empty workspace).
  - This contrasts with `Escape`, which is a client-side GTK window key handler and requires the QuickPeek preview window to be focused to close it.

- **Known v1 Wart: First Press After Login**:
  - When the QuickPeek daemon is not yet running on the D-Bus session bus, pressing `Mod+Space` starts `quickpeek` in **service mode** (`ROLE DAEMON`). In service mode, it acquires the bus name and prepares to serve requests, but displays no window.
  - As a result, the very first keypress after a fresh login starts the daemon; subsequent keypresses toggle previews.
  - In Phase 7, a `systemd` user service unit will launch the daemon automatically at login, eliminating this initial wart.

- **Stopping the Daemon**:
  - To stop the running QuickPeek background daemon manually:
    ```bash
    pkill -x quickpeek
    ```
  - Or via D-Bus directly:
    ```bash
    gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.Quit
    ```
  - Full lifecycle service management will be provided via `systemctl --user` in Phase 7.

- **Bare `Space` Key (Unmodified Space)**:
  - Binding bare `Space` globally at the compositor level breaks normal typing in text fields, editors, and terminal emulators across the desktop.
  - Unmodified `Space` preview is explicitly a Phase 8 stretch experiment exploring conditional window-context filtering and synthetic key re-injection.
  - For v1 core reliability, `Mod+Space` / `Super+Space` is the authoritative, conflict-free shortcut.
