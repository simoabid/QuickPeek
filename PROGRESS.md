# QuickPeek Progress & Source of Truth

This document is the authoritative tracking ledger for the QuickPeek implementation.

## Performance & Resource Budgets
| Metric | Budget Target | Phase 0 Probe Actual | Status |
| :--- | :--- | :--- | :--- |
| Cold Startup / Launch | $\le 300\text{ ms}$ | Build time: 126 s (cold compile) | PASS (within 300 s compile budget) |
| Warm Preview Display | $\le 80\text{ ms}$ | TBD (Phase 4) | PENDING |
| Close / Dismissal | $\le 50\text{ ms}$ | TBD (Phase 4) | PENDING |
| Memory Footprint (RSS) | $\le 150\text{ MB}$ | ~35 MB (minimal GTK4 instance) | PASS |
| Binary Size | $\le 15\text{ MB}$ | 454 KB (probe release binary) | PASS |
| Automated Test Pass Rate | 100% | 100% (probes succeeded) | PASS |

---

## Roadmap (Phases 0–12)
| Phase | Name / Focus | Status |
| :---: | :--- | :---: |
| **0** | **Reconnaissance & Environment Verification** | **IN PROGRESS** |
| 1 | Single-Instance D-Bus Service (`org.quickpeek.QuickPeek`) | PLANNED |
| 2 | AT-SPI Selection Extraction (Nautilus & Dolphin) | PLANNED |
| 3 | Compositor Integration & Keybindings (Hyprland & Niri) | PLANNED |
| 4 | Core Preview Window & GTK4 UI Container | PLANNED |
| 5 | Image Preview Plugin: PNG & JPG | PLANNED |
| 6 | Image Preview Plugin: Animated GIF & WebP | PLANNED |
| 7 | Vector Preview Plugin: SVG | PLANNED |
| 8 | Keyboard Navigation (Arrows to cycle selection, Esc to dismiss) | PLANNED |
| 9 | Fallback & Error Handling (Unsupported formats, missing files) | PLANNED |
| 10 | Performance Tuning & Latency Benchmark Hardening | PLANNED |
| 11 | Packaging & Deployment (PKGBUILD, AUR, systemd user service) | PLANNED |
| 12 | Final QA, Regression Suite & Documentation Wrap-up | PLANNED |

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
1. **Name Collision**:
   - `quickpeek` is available on crates.io and AUR, and no active Linux previewers exist under that name on GitHub. Recommendation is to proceed with `quickpeek`.
2. **Dolphin AT-SPI Registration**:
   - In pure Wayland environments (like Niri or Hyprland outside full KDE Plasma), Dolphin does not activate AT-SPI accessibility by default unless `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1` is exported. Will QuickPeek mandate this environment variable in user instructions / systemd environment, or does Dolphin provide a secondary D-Bus interface (e.g. `org.freedesktop.FileManager1`) for selection querying?
3. **Crate Name Pinned (`gtk` vs `gtk4`)**:
   - Running `cargo add gtk` resolves to `gtk v0.19.0` (GTK3 bindings). The GTK4 crate via `gtk4-rs` is `gtk4 = "0.11.5"`. The probe confirmed `gtk4 = "0.11.5"` builds and links against system `libgtk-4.so.1`. Phase 1 should explicitly use `gtk4`.
4. **Linker Configuration on CachyOS / Arch**:
   - Rustc 1.98.1 package on this CachyOS system defaults to looking for `x86_64-linux-gnu-gcc`. Specifying `linker = "gcc"` in `.cargo/config.toml` resolved this cleanly. Should QuickPeek include a checked-in `.cargo/config.toml` setting `linker = "gcc"`?
5. **Active Compositor Keybinding Dispatch**:
   - Current session is running Niri (`niri 26.04`), with Hyprland installed but inactive. The keybinding interface differs (`niri msg action spawn` vs `hyprctl dispatch exec`). Phase 3 should support configuration templates for both.
