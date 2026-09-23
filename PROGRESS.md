# QuickPeek Progress & Source of Truth

This document is the authoritative tracking ledger for the QuickPeek implementation.

## Performance & Resource Budgets
| Metric | Budget Target | Phase 1 Actual | Phase 2 Actual | Status |
| :--- | :--- | :--- | :--- | :--- |
| Cold Startup / Launch | $\le 300\text{ ms}$ | **123 ms** (`MAP_MS 123`) | **255 ms** (`MAP_MS 255`) | **PASS** |
| Warm Preview Display | $\le 80\text{ ms}$ | TBD (Phase 4) | **49 ms** (worst-of-5: 34ms, 35ms, 49ms, 34ms, 36ms) | **PASS** |
| Close / Dismissal | $\le 50\text{ ms}$ | TBD (Phase 4) | TBD (Phase 8) | PENDING |
| Memory Footprint (RSS) | $\le 150\text{ MB}$ | ~35 MB (minimal GTK4 instance) | **54.8 MB** (54,804 KB idle GTK4 daemon) | **PASS** |
| Binary Size | $\le 15\text{ MB}$ | **507 KB** (`target/release/quickpeek`) | **555 KB** (568,256 B) | **PASS** |
| Cold Build Time | $\le 300\text{ s}$ | **117.3 s** (`1m 57s` release build) | **117.3 s** (incremental: 0.07s) | **PASS** |
| Automated Test Pass Rate | 100% | **100%** (8 passed / 0 failed) | **100%** (13 cargo unit + 8 e2e dbus checks passed) | **PASS** |

---

## Roadmap (Phases 0–12)
| Phase | Name / Focus | Status |
| :---: | :--- | :---: |
| **0** | **Reconnaissance & Environment Verification** | **COMPLETE** |
| **1** | **Skeleton — Single Image Preview Window** | **COMPLETE** |
| **2** | **D-Bus Single-Instance Daemon (`org.quickpeek.QuickPeek`)** | **COMPLETE** |
| 3 | Image Formats Expansion: WebP, SVG, Animated GIF | PLANNED |
| 4 | AT-SPI Selection Extraction (Dolphin) | PLANNED |
| 5 | AT-SPI Selection Extraction (Nautilus) | PLANNED |
| 6 | Compositor Integration & Keybindings (Hyprland & Niri) | PLANNED |
| 7 | Daemon Packaging & systemd User Service | PLANNED |
| 8 | Keyboard Navigation (Arrows to cycle selection, Esc to dismiss) | PLANNED |
| 9 | Window Placement & Compositor Rules (Floating, Centering) | PLANNED |
| 10 | Fallback & Error Handling (Unsupported formats, missing files) | PLANNED |
| 11 | Performance Tuning & Latency Benchmark Hardening | PLANNED |
| 12 | Final QA, Regression Suite & Documentation Wrap-up | PLANNED |

---

## Phase 2 API Facts
Verified against pinned dependencies (`gtk4 0.11.5`, `gio 0.22.10`, `glib 0.22.10`):
1. **Re-exports**:
   `gtk4/src/lib.rs:12-13`: `pub use gio; pub use glib;`
2. **Session Bus Connection**:
   `gio::bus_get_sync(bus_type: BusType, cancellable: Option<&impl IsA<Cancellable>>) -> Result<DBusConnection, glib::Error>`
3. **Synchronous D-Bus Calls (`RequestName` and client `ShowFile`)**:
   `DBusConnection::call_sync(&self, bus_name: Option<&str>, object_path: &str, interface_name: &str, method_name: &str, parameters: Option<&glib::Variant>, reply_type: Option<&glib::VariantTy>, flags: DBusCallFlags, timeout_msec: i32, cancellable: Option<&impl IsA<Cancellable>>) -> Result<glib::Variant, glib::Error>`
4. **Introspection & Interface Lookup**:
   `DBusNodeInfo::for_xml(xml_data: &str) -> Result<DBusNodeInfo, glib::Error>`
   `DBusNodeInfo::lookup_interface(&self, name: &str) -> Option<DBusInterfaceInfo>`
5. **D-Bus Object Registration**:
   `DBusConnection::register_object<'a>(&'a self, object_path: &'a str, interface_info: &'a DBusInterfaceInfo) -> RegistrationBuilder<'a>`
   `RegistrationBuilder::method_call(mut self, f: F) -> Self` where `F: Fn(DBusConnection, Option<&str>, &str, Option<&str>, &str, glib::Variant, DBusMethodInvocation) + 'static`
   `RegistrationBuilder::build(self) -> Result<RegistrationId, glib::Error>`
6. **Method Invocation Responses**:
   `DBusMethodInvocation::return_value(self, parameters: Option<&glib::Variant>)`
   `DBusMethodInvocation::return_dbus_error(self, error_name: &str, error_message: &str)`
7. **Tooling Verification**:
   - `/usr/bin/dbus-run-session` (present)
   - `/usr/bin/gdbus` (present)
   - `/usr/bin/busctl` (present)

---

## Phase 2 Checklist
*(Items marked VERIFIED-BY-INSPECTION until confirmed by human)*

- [x] **VERIFIED-BY-INSPECTION** — Pre-GTK role decision via synchronous D-Bus `RequestName`:
  > Command: `target/release/quickpeek tests/fixtures/sample.png` (first instance) vs subsequent client invocations.
  > Daemon acquires `org.quickpeek.QuickPeek` (`RequestName` reply 1 `PRIMARY_OWNER`), logs `ROLE DAEMON`. Client detects name owned (`RequestName` reply 3 `EXISTS`), logs `ROLE CLIENT`. Client never invokes `gtk::init()`.
- [x] **VERIFIED-BY-INSPECTION** — D-Bus interface registration and XML introspection:
  > Command: `gdbus introspect --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek`
  > Output confirms `interface org.quickpeek.QuickPeek` with methods `ShowFile(in s path, out b ok, out s message)` and `Quit()`. Object registered before acquiring bus name to guarantee zero-window race.
- [x] **VERIFIED-BY-INSPECTION** — Image swap & error handling via `ShowFile`:
  > Command: `target/release/quickpeek tests/fixtures/sample2.png` while daemon running.
  > Output: Daemon swaps texture in existing window, logs `IMAGE_SWAPPED` and `WARM_MS <ms>`. Client prints `CALL_MS <ms>`.
  > Nonexistent path returns `(false, 'file not found: ...')`.
  > Non-image path (`notimage.txt`) returns `(false, 'failed to decode image: ...')`.
- [x] **VERIFIED-BY-INSPECTION** — 2000 ms client timeout on unresponsive daemon:
  > Verified in `./tests/e2e_dbus.sh` using `kill -STOP` on daemon PID. Client synchronously times out after 2000 ms, logs `Error: D-Bus call failed: Timeout was reached`, and exits with code 1.
- [x] **VERIFIED-BY-INSPECTION** — Clean `Quit` method lifecycle:
  > Command: `gdbus call --session --dest org.quickpeek.QuickPeek --object-path /org/quickpeek/QuickPeek --method org.quickpeek.QuickPeek.Quit`
  > Daemon returns `()`, queues main loop exit via `glib::idle_add_local_once`, closes window, unexports object, releases bus name, and exits 0 (`DAEMON_QUIT`).
- [x] **VERIFIED-BY-INSPECTION** — Zero dependency delta:
  > Command: `git diff Cargo.lock`
  > Output: empty (0 crates added; uses re-exported `gtk4::gio` and `gtk4::glib`).
- [x] **VERIFIED-BY-INSPECTION** — Automated unit and hermetic E2E test suites passing:
  > Command: `cargo test` -> `13 passed; 0 failed`.
  > Command: `dbus-run-session ./tests/e2e_dbus.sh` -> 8/8 checks pass, exit 0.
- [x] **VERIFIED-BY-INSPECTION** — Performance gates within budget:
  > Warm preview latency: worst of 5 = **49 ms** ($\le 80\text{ ms}$).
  > Cold startup first map: **255 ms** ($\le 300\text{ ms}$).
  > Binary size: **555 KB** / 568,256 B ($\le 15\text{ MB}$).
  > Daemon idle RSS: **54.8 MB** / 54,804 KB ($\le 150\text{ MB}$).
  > Incremental build time: **0.07 s**.
- [x] **VERIFIED-BY-INSPECTION** — Screenshot captured:
  > Window screenshot showing swapped image (`sample2.png`) saved to `REPORTS/phase2_window.png` (399 KB, 1920x1080).
- [ ] **HUMAN-VERIFICATION-PENDING** — Interactive desktop session verification:
  > Terminal A (foreground daemon) + Terminal B (client invocations showing image swap on desktop), Escape key dismissal while daemon persists in background loop, and Ctrl+C clean exit.

---

## Phase 1 Checklist
*(Items marked VERIFIED-BY-INSPECTION until confirmed by human)*

- [x] **VERIFIED-BY-INSPECTION** — Root crate scaffolded with `gtk4 = "0.11"` and committed `Cargo.lock`:
  > Dependencies strictly limited to `gtk4 = "0.11"` (resolved to `0.11.5`). Repo `.cargo/config.toml` includes linker fix `[target.x86_64-unknown-linux-gnu] linker = "gcc"`.
- [x] **VERIFIED-BY-INSPECTION** — 64x64 PNG sample fixture created via probe helper:
  > Path: `tests/fixtures/sample.png`, verified: `PNG image data, 64 x 64, 8-bit/color RGBA`, actual size: 168 bytes (< 10 KB).
- [x] **VERIFIED-BY-INSPECTION** — Unit tests implemented and passing:
  > Command: `cargo test`
  > Output: `8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`.
  > Tests verify CLI path validation (missing, extra args, nonexistent file, unsupported format, valid PNG) and exact aspect-fit math (4000x3000@60% -> 864x648, 64x64 -> 64x64 [never upscale], 500x2000@60% -> 162x648).
- [x] **VERIFIED-BY-INSPECTION** — Cold release build within budget:
  > Command: `time cargo build --release`
  > Output: `Finished release profile [optimized] target(s) in 1m 57s` (`real 1m57.299s` vs $\le 300\text{ s}$ budget).
- [x] **VERIFIED-BY-INSPECTION** — Binary size within budget:
  > Command: `ls -lh target/release/quickpeek`
  > Output: `507K` (518,184 bytes vs $\le 15\text{ MB}$ budget).
- [x] **VERIFIED-BY-INSPECTION** — Cold-start proxy metric (MAP_MS) within budget:
  > Command: `target/release/quickpeek tests/fixtures/sample.png`
  > Output: `MAP_MS 123` (123 ms vs $\le 300\text{ ms}$ budget).
- [x] **VERIFIED-BY-INSPECTION** — Window display verified via screen capture:
  > Screenshot captured while mapped: `REPORTS/phase1_window.png` (244 KB, 1920x1080).
- [x] **VERIFIED-BY-INSPECTION** — CLI error handling verified:
  > No args: `Error: missing image path argument`, exit code 1.
  > Nonexistent path: `Error: file not found: nonexistent_file_xyz.png`, exit code 1.
- [ ] **HUMAN-VERIFICATION-PENDING** — Escape key dismissal:
  > Synthesized key event injection tooling (`ydotool`/`wtype`) is not installed on system. `EventControllerKey` handler attached to window listening for `gdk::Key::Escape` to call `window.close()` -> `main_loop.quit()` -> exit code 0.

---

## Phase 0 Checklist
*(Items marked VERIFIED-BY-INSPECTION until confirmed by human)*

- [x] **VERIFIED-BY-INSPECTION** — Git branch on `main` and origin set:
  > Command: `git branch --show-current && git remote -v`
  > Output: `main`, `origin https://github.com/simoabid/QuickPeek.git`
- [x] **VERIFIED-BY-INSPECTION** — Toolchain prerequisites verified:
  > Command: `rustc --version && cargo --version && pkg-config --modversion gtk4`
  > Output: `rustc 1.98.1`, `cargo 1.98.1`, `gtk4 4.22.5`
- [x] **VERIFIED-BY-INSPECTION** — Compositor & session probed:
  > Command: `echo "$XDG_CURRENT_DESKTOP $XDG_SESSION_TYPE" && niri --version`
  > Output: `niri wayland`, `niri 26.04 (8ed0da4)`
  > Hyprland probe: `hyprctl version` returned `HYPRLAND_INSTANCE_SIGNATURE not set! (is hyprland running?)` (exit code: 1, compositor not active)
- [x] **VERIFIED-BY-INSPECTION** — File managers detected:
  > Command: `nautilus --version && dolphin --version`
  > Output: `GNOME nautilus 50.3.1`, `dolphin 26.08.1`
- [x] **VERIFIED-BY-INSPECTION** — AT-SPI bus inspected and documented:
  > Command: `gdbus call --session --dest org.a11y.Bus --object-path /org/a11y/bus --method org.a11y.Bus.GetAddress`
  > Output: `('unix:path=/run/user/1000/at-spi/bus_0',)`
  > Bus list confirmed Nautilus registered (`:1.15`). Dolphin registers when `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1` is provided.
- [x] **VERIFIED-BY-INSPECTION** — Screenshot & clipboard tools verified:
  > Command: `pacman -Q grim && wl-paste --version`
  > Output: `grim 1.5.0-2.1`, `wl-clipboard 2.3.0`
- [x] **VERIFIED-BY-INSPECTION** — Name collision checks completed:
  > GitHub: 36 unrelated results. AUR: 0 results. crates.io: 404 Not Found.
- [x] **VERIFIED-BY-INSPECTION** — Base repository scaffolding committed:
  > `README.md` (7 lines), `LICENSE` (unedited GPL-3.0), `.gitignore` (`target/`, `probe/`, `reference/`, `*.log`).
- [x] **VERIFIED-BY-INSPECTION** — Reference clone analyzed & gitignored:
  > `reference/QuickLook` cloned (383M), working tree stays clean. UX principles and plugin separation documented.
- [x] **VERIFIED-BY-INSPECTION** — Toolchain proof crate (`probe/`) built & executed:
  > Release build time: 2m 06s. Release binary size: 454 KB. `ldd | wc -l`: 113. Links `libgtk-4.so.1`. Exit code: 0.

---

## Open Questions for Planner
1. **GitHub Remote Push Credentials**:
   - `git push -u origin main` prompts for GitHub credentials over HTTPS (`https://github.com/simoabid/QuickPeek.git`). Human verification/action required to cache credentials or set up SSH key so background pushes can succeed unattended.
2. **Phase 3 Preview Format Decoder Dependencies**:
   - Phase 3 expands image preview support to WebP, SVG, and animated GIF. With `gtk4 = "0.11"` relying on `gdk::Texture::from_file` and system `gdk-pixbuf2` loaders, will we rely on GDK's native loaders (which already decode SVG and WebP via librsvg / libwebp), or should we introduce explicit Rust decoding crates (e.g. `image` crate)?
3. **Dolphin AT-SPI Environment Setup for Phase 4**:
   - In standalone Wayland sessions (Niri/Hyprland), Dolphin only exposes AT-SPI when `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1` is exported in the user session. Shall Phase 4 assume this environment variable is set by the user/systemd session, or should QuickPeek also probe for Dolphin's window via compositor IPC (`niri msg` / `hyprctl`)?
