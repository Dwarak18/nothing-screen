# NothingCast — Execution Plan

Derived from [PLAN.md](PLAN.md) v1. Stack decision: **Tauri 2 + Rust** desktop shell,
Vite + TypeScript UI, bundled `adb` + `scrcpy-server`, `mdns-sd` for discovery.

Status legend: `[ ]` todo · `[~]` in progress · `[x]` done · `[!]` blocked

---

## Phase M0 — Tech Spike (target: 2 weeks)

Goal: prove the pipeline headlessly before any UI. Exit criteria from PLAN.md:
mDNS-discover → adb pair → 1080p60 Wi-Fi stream; latency measured via on-screen stopwatch.

### M0.1 Environment & tooling
- [~] Install Rust toolchain (`rustup`, MSVC target on Windows) + Tauri 2 prerequisites (WebView2) — cargo 1.98 verified working on Windows host
- [x] Vendor platform binaries: `adb.exe` (platform-tools), `scrcpy-server` jar — stored under `src-tauri/binaries/`; resolved via bundled-path lookup in `adb.rs`
- [ ] CI skeleton: GitHub Actions — lint + clippy + `cargo check` + frontend typecheck on every PR

### M0.2 ADB transport layer (Rust module: `src-tauri/src/adb.rs`)
- [~] Spawn/manage `adb` server lifecycle (`start-server`, kill on exit) — `ensure_server()` wired into connect/pair commands; exit-kill pending
- [x] `adb pair <ip:port> <code>` wrapper (Android 11+ wireless pairing) — exposed as `pair_device` command
- [~] `adb connect` / `disconnect` / `devices -l` parsing into typed structs — connect + devices done; `disconnect` wrapper pending
- [ ] mDNS-assisted connect: read `_adb-tls-connect._tcp` TXT records for auto-connect

### M0.3 Discovery (Rust module: `src-tauri/src/mdns.rs`)
- [x] Integrate `mdns-sd` crate; browse `_adb-tls-pairing._tcp` and `_adb-tls-connect._tcp`
- [x] Device list events pushed to frontend (`phone-found` / `phone-lost`) — background pump thread emits Tauri events
- [x] Distinguish "needs pairing" vs "ready to connect" states per device

### M0.4 Stream pipeline spike (Rust module: `src-tauri/src/scrcpy.rs`)
- [x] Push `scrcpy-server` to device (`adb push`), forward socket, launch `app_process`
      Verified live on Nothing A069 / Android 16 over Wi-Fi. Gotchas encoded in `scrcpy.rs`:
      whole command must be ONE shell string (adb shell v2 execs args directly → CLASSPATH dropped);
      Windows needs `forward tcp:27183 localabstract:scrcpy` (no host abstract sockets);
      `tunnel_forward=true` so the server listens and our decoder dials 127.0.0.1:27183;
      re-push jar every session (/data/local/tmp gets cleared).
- [ ] Parse scrcpy control/demuxer protocol handshake (device meta, codec config)
- [ ] Decode path decision: raw H.264 → surface render. Spike both:
      - [ ] Option 1: FFmpeg (via `ffmpeg-sidecar` or vendored dll) → texture
      - [ ] Option 2: hardware decode via WebView-independent native window
- [ ] Control injection: mouse/touch events → scrcpy control socket
- [ ] Audio forwarding (Android 11+): second scrcpy audio channel → sink to output device

### M0.5 Latency measurement harness
- [ ] Stopwatch-on-screen method: phone displays running ms clock; screenshot both screens; compute glass-to-glass delta
- [ ] Scripted run matrix: {5 GHz vs 2.4 GHz} × {H.264 vs H.265} × {720p60 vs 1080p60}
- [ ] Record results in `docs/spike-results.md`; go/no-go against ≤80 ms target

**M0 exit gate:** demo script runs end-to-end from CLI; latency numbers documented; decode option chosen.

---

## Phase M1 — Windows MVP (target: 4–6 weeks)

### M1.1 App shell
- [ ] Tauri window: tray icon + quick-connect menu, single main window
- [ ] State machine: `idle → discovering → pairing → connecting → streaming` with error branches (Rust `state.rs`, emitted as events)
- [ ] Saved devices store (JSON in app-data dir): last IP, name, quality preset

### M1.2 Onboarding flow (UI)
- [ ] Guided "Enable wireless debugging" walkthrough — Nothing OS-specific screenshots (capture on Nothing Phone 2a/3); generic docs link works today
- [~] 6-digit pairing-code screen wired to `adb pair` — inline per-device code entry shipped; dedicated guided screen pending
- [ ] Health checks: same-subnet detection, 2.4 GHz warning, firewall port probe
- [ ] Empty state design per PLAN.md §8 (illustration + two actions, never bare text)

### M1.3 Mirror window & control
- [ ] Aspect-correct resizable window, rotate-with-phone, fullscreen toggle
- [ ] Input injection at display refresh rate; keyboard; clipboard sync both ways
- [ ] Drag-and-drop file push (`adb push`)
- [ ] Quality presets: Smooth 720p60 / Balanced 1080p60 / Sharp native H.265

### M1.4 Audio + HUD
- [ ] Audio forwarding enabled by default on Android 11+; ±40 ms sync check
- [ ] Connection HUD: bitrate, fps, latency estimate; "Your Wi-Fi is the bottleneck" diagnostic card

### M1.5 Installer & firewall
- [ ] NSIS/WiX installer; adds Windows Firewall inbound rule with user consent at first run
- [ ] Code signing cert procured; installer tested on clean Win11 VM
- [ ] Crash reporting + opt-in telemetry scaffolding (respect §9 metrics)

**M1 exit gate:** stranger test — a Nothing owner pairs and mirrors unaided in ≤90 s; tap-to-picture ≤2 s warm.

---

## Phase M2 — Cross-platform (target: 4 weeks)
- [ ] macOS: notarization, Bonjour verification, Retina scaling
- [ ] Linux: udev rules packaging (`.deb`/`.rpm`/AUR), X11+Wayland capture caveats documented
- [ ] Platform CI matrices; binary size budget (<40 MB installed excluding adb)

## Phase M3 — Polish pass (target: 3 weeks)
- [ ] Cold-start tap-to-picture ≤2 s (pre-warm adb server, cached device route)
- [ ] Auto-reconnect on sleep/wake/network change
- [ ] Full error-state audit: locked phone, wrong network, firewall blocked — each with fix action
- [ ] Latency/fps adaptive bitrate tuning

## Phase M4 — Private beta (target: ongoing)
- [ ] Beta distribution channel + feedback loop (50 Nothing owners)
- [ ] Telemetry dashboard for §9 metrics (median/p95 tap-to-picture, session latency)
- [ ] Weekly compat matrix run against current Nothing OS releases

---

## Standing decisions
| Decision | Choice | Rationale |
|---|---|---|
| Architecture | PLAN.md Option A → C | scrcpy engine quality; helper app later |
| Stack | Tauri 2 + Rust, Vite + TS UI | small binaries, native perf, one codebase ×3 OS |
| mDNS crate | `mdns-sd` | pure-Rust, no system Bonjour dependency |
| Decode | decided at M0 gate | FFmpeg-texture vs native-window spike |
| Rename | pending | "NothingCast" trademark risk — pick new name before any public artifact |

## Risks watched weekly
1. Wi-Fi variance → adaptive bitrate + diagnostics (M1.4)
2. Developer Options friction → guided flow investment (M1.2)
3. Nothing OS breakage → compat matrix (M4)
4. Windows firewall/AV → installer rule + docs (M1.5)
