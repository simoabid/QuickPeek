# QuickPeek Live Dolphin Selection & AT-SPI Resolution

QuickPeek integrates with KDE Dolphin via the Linux desktop accessibility bus (AT-SPI2) to preview whichever file is currently selected when you press `Mod+Space`.

---

## 1. End-User Dolphin Setup

Qt applications only register with the accessibility bus when accessibility is enabled in the Qt runtime.

### Required Environment Variable
The environment variable `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1` **must be set before Dolphin starts** (Qt reads accessibility flags strictly during process startup).

### Shell Configuration (Fish)
In your `~/.config/fish/config.fish`:
```fish
set -gx QT_LINUX_ACCESSIBILITY_ALWAYS_ON 1
```

### Shell Configuration (Bash / Zsh)
In your `~/.bashrc` or `~/.zshrc`:
```bash
export QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1
```

### Manual Testing
To run Dolphin with accessibility enabled directly from a terminal:
```bash
QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1 dolphin /path/to/folder &
```

> [!NOTE]
> Global session-wide environment wiring (via `~/.config/environment.d/`) will arrive in **Phase 7**. Setting the variable in your shell profile or launching Dolphin with the variable is the recommended workflow for Phase 4.

---

## 2. Selection Resolution Order

Whenever an open trigger occurs (either `Mod+Space` on a running daemon, or a cold-start `quickpeek` with no arguments), QuickPeek resolves the preview target using this strict 3-tier hierarchy:

1. **Live AT-SPI Selection (Active Dolphin Window)**:
   - QuickPeek queries the AT-SPI accessibility bus (`org.a11y.Bus`) within a hard **150 ms deadline**.
   - It locates the active, focused top-level Dolphin window (`ROLE_FRAME` holding `ATSPI_STATE_ACTIVE`).
   - It inspects the focused view (`ROLE_LIST` / `ROLE_TABLE`) and extracts the primary selected item (`GetSelectedChild(0)`).
   - It derives the absolute directory from the window caption or location bar and constructs the full file path.
   - On success, QuickPeek emits `SELECTION_LIVE <path>` and persists this path to disk.

2. **In-Memory Selection Fallback**:
   - If Dolphin is unfocused, absent, or has no file selected, QuickPeek falls back to the daemon's in-memory `current_selection`.
   - Emits `SELECTION_FALLBACK memory`.

3. **Persisted Selection Fallback**:
   - If in-memory selection is empty or stale, QuickPeek falls back to `$XDG_STATE_HOME/quickpeek/last_selection` (`~/.local/state/quickpeek/last_selection`).
   - Emits `SELECTION_FALLBACK persisted`.

4. **Empty / No Selection**:
   - If all three sources are empty or non-existent, QuickPeek emits `SELECTION_FALLBACK none` and returns a clean error (`Error: no file to preview`).

> [!IMPORTANT]
> **Short-Circuit on Close**: If the preview window is already open, pressing `Mod+Space` immediately closes the window (`TOGGLE_CLOSED`). **Zero AT-SPI queries run** during close, ensuring sub-5ms close latency.

> [!NOTE]
> **First Use Carry-Forward**: On fresh systems where Dolphin is not running or no file has ever been selected, pressing `Mod+Space` falls back through the hierarchy and exits cleanly with `Error: no file to preview` until a selection source exists.

---

## 3. Troubleshooting & Diagnostics

QuickPeek logs diagnostic tokens to stdout/stderr or the user systemd journal:

| Diagnostic Token | Meaning | Next Step / Remedy |
| :--- | :--- | :--- |
| `ATSPI_UNAVAILABLE no active Dolphin window found` | Dolphin is either closed or not the currently focused window. | Normal behavior when working outside Dolphin; QuickPeek falls back to your last previewed image. |
| `ATSPI_UNAVAILABLE no item selected in active Dolphin window` | The active Dolphin window has no file currently highlighted. | Click or use arrow keys in Dolphin to select a file. |
| `ATSPI_UNAVAILABLE Dolphin application not found on accessibility bus` | Dolphin was started without `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1`. | Quit Dolphin completely (`pkill -x dolphin`) and restart it with the environment variable set. |
| `ATSPI_UNAVAILABLE org.a11y.Bus GetAddress failed` | The AT-SPI user bus daemon is not active in the current session. | Check `systemctl --user status at-spi-dbus-bus.service` or restart it. |
| `ATSPI_UNAVAILABLE AT-SPI deadline exceeded` | AT-SPI bus query exceeded the 150 ms deadline. | Automatic fail-open; QuickPeek safely fell back to the last previewed file without stalling the user UI. |

---

## 4. Performance Budgets

- **AT-SPI Hard Deadline**: 150 ms (fail-open to fallback).
- **With-Lookup Toggle Budget**: $\le 250\text{ ms}$ (typical live measurements: 16–26 ms for AT-SPI walk, ~60–80 ms total toggle).
- **Fallback Toggle Gate**: $\le 80\text{ ms}$ (typical live measurements: 2–5 ms).
- **Toggle Close**: $\le 5\text{ ms}$ (no AT-SPI queries executed).
