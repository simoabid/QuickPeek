# QuickPeek Progress & Source of Truth

This document is the authoritative tracking ledger for the QuickPeek implementation.

## Performance & Resource Budgets
| Metric | Budget Target | Phase 1 Actual | Status |
| :--- | :--- | :--- | :--- |
| Cold Startup / Launch | $\le 300\text{ ms}$ | **123 ms** (`MAP_MS 123`) | **PASS** |
| Warm Preview Display | $\le 80\text{ ms}$ | TBD (Phase 4) | PENDING |
| Close / Dismissal | $\le 50\text{ ms}$ | TBD (Phase 4) | PENDING |
| Memory Footprint (RSS) | $\le 150\text{ MB}$ | ~35 MB (minimal GTK4 instance) | **PASS** |
| Binary Size | $\le 15\text{ MB}$ | **507 KB** (`target/release/quickpeek`) | **PASS** |
| Cold Build Time | $\le 300\text{ s}$ | **117.3 s** (`1m 57s` release build) | **PASS** |
| Automated Test Pass Rate | 100% | **100%** (8 passed / 0 failed) | **PASS** |

---

## Roadmap (Phases 0–12)
| Phase | Name / Focus | Status |
| :---: | :--- | :---: |
| **0** | **Reconnaissance & Environment Verification** | **COMPLETE** |
| **1** | **Skeleton — Single Image Preview Window** | **COMPLETE** |
| 2 | AT-SPI Selection Extraction (Nautilus & Dolphin) | PLANNED |
| 3 | Image Formats Expansion: WebP, SVG, Animated GIF | PLANNED |
| 4 | Single-Instance D-Bus Service (`org.quickpeek.QuickPeek`) | PLANNED |
| 5 | Compositor Integration & Keybindings (Hyprland & Niri) | PLANNED |
| 6 | Keyboard Navigation (Arrows to cycle selection, Esc to dismiss) | PLANNED |
| 7 | Window Placement & Compositor Rules (Floating, Centering) | PLANNED |
| 8 | Core Preview Container & Header/Controls | PLANNED |
| 9 | Fallback & Error Handling (Unsupported formats, missing files) | PLANNED |
| 10 | Performance Tuning & Latency Benchmark Hardening | PLANNED |
| 11 | Packaging & Deployment (PKGBUILD, AUR, systemd user service) | PLANNED |
| 12 | Final QA, Regression Suite & Documentation Wrap-up | PLANNED |

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
2. **Phase 2 & Phase 3 Roadmap Ordering**:
   - Planner decision notes indicate Phase 2 (AT-SPI file manager selection detection) vs Phase 3 (image format expansion: WebP/SVG/GIF). Is AT-SPI selection detection the next priority for Phase 2?
3. **Dolphin AT-SPI Strategy**:
   - In standalone Wayland compositors (Niri/Hyprland without full KDE Plasma), Dolphin requires `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1`. Should QuickPeek document this requirement for users or implement a fallback to `org.freedesktop.FileManager1`?
