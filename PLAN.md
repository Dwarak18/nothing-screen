# NothingCast — Plan
Wireless screen mirroring from Nothing Phone to Windows / Linux / Mac with iPhone-grade smoothness.

Status: DRAFT v1 · Date: 2026-08-25

---

## 1. Problem & Vision

Nothing Phone users have no native way to mirror their phone to a desktop the way an
iPhone mirrors to a Mac via AirPlay. Existing tools (scrcpy) are smooth but
developer-oriented: CLI, manual `adb` setup, no discovery, ugly windows. Consumer
tools (Vysor, AirDroid) are laggy, ad-laden, or cloud-routed.

**Vision:** pick up your Nothing Phone, tap once, and your screen appears on your
desktop in under 2 seconds — 60fps, low latency, zero cables, zero cloud.

**The bar ("iPhone-like smoothness"):**
| Metric | Target |
|---|---|
| Glass-to-glass latency (Wi-Fi 5/6, same LAN) | ≤ 80 ms |
| Frame rate | 60 fps sustained |
| Time from tap → visible picture | ≤ 2 s |
| Setup friction | First-time pair ≤ 90 s; every time after = one click |
| Audio | Synced within ±40 ms of video |

---

## 2. Target Users

1. **Primary:** Nothing Phone owners who want their phone on the big screen —
   gaming, messaging at a desk, presentations, streaming.
2. **Secondary:** developers/testers who want scrcpy quality without the CLI.
3. **Tertiary:** anyone with Android 11+ later (architecture must not be
   Nothing-specific).

## 3. Competitive Landscape

| Product | Smoothness | Wireless | Friction | Price |
|---|---|---|---|---|
| scrcpy (free, OSS) | Excellent (~35–70ms) | Yes, but manual adb tcpip dance | High (CLI, dev options) | Free |
| Vysor | Mediocre | Yes | Medium | Freemium |
| AirDroid | Mediocre, often cloud-routed | Yes | Medium | Freemium/ads |
| Windows "Phone Link" (Android) | OK, not real-time | Yes | Low | Free, Windows-only, limited |

**Our wedge:** scrcpy-class smoothness + AirPlay-class UX. Nobody owns that quadrant today.

---

## 4. Architecture Decision

### Option A — scrcpy engine, consumer shell (RECOMMENDED for MVP)
Wrap the proven scrcpy pipeline (device-side hardware H.264/H.265 encode via
MediaCodec + SurfaceFlinger capture, streamed over ADB transport, native host decode)
inside a polished desktop app:

- Desktop apps (Windows/Linux/Mac) bundle `scrcpy-server` + `adb`.
- Discovery via mDNS/Bonjour (`_adb-tls-connect._tcp`, Android 11+ wireless debugging)
  so the phone shows up automatically.
- First-time pairing: guided flow using Android's wireless-debugging pairing code
  (no cable needed on Android 11+). Fallback: USB plug-in once.
- After pairing: one click connects. App auto-issues `adb connect`, launches stream,
  remembers the device.

**Pros:** lowest latency available on Android (~35–70ms wired; near that on clean 5 GHz Wi-Fi),
hardware encoding on the phone (near-zero battery/CPU impact), battle-tested,
works with zero phone-side app install.
**Cons:** requires Developer Options + wireless debugging enabled once (guided);
ADB transport adds a small overhead vs raw socket.

### Option B — Companion Android app (MediaProjection + WebRTC/raw RTP)
Phone app captures its own screen (MediaProjection foreground service), encodes with
MediaCodec, streams H.264 over a LAN socket/WebRTC to desktop receivers.

**Pros:** no Developer Options at all — pure consumer onboarding; full control of pipeline.
**Cons:** MediaProjection adds capture+encode overhead and typically +30–100ms vs
SurfaceFlinger path; consent dialog every session (or persistent-notification workarounds);
more battery; WebRTC stacks add complexity. This is how laggy competitors do it.

### Option C — Hybrid (Phase 2 target)
Keep scrcpy engine for quality, add our own thin phone-side helper installed via ADB
on first pair (like scrcpy does transparently) to enable: reconnection without
developer options, notification mirroring, file drop, virtual display features.
Long term, replace ADB transport with a direct TCP/QUIC socket once paired to shave
transport overhead.

**Decision: Ship MVP as A. Evolve toward C. Do NOT build B as primary path.**

---

## 5. MVP Scope

### Desktop app (all three OS)
- Auto-discovery of phones on LAN (mDNS scan + saved devices).
- One-click connect / disconnect. Device picker when multiple phones found.
- Mirror window: resizable, aspect-correct, rotate-with-phone, fullscreen.
- Control: mouse/touch injection, keyboard, clipboard sync both ways, drag-and-drop file push.
- Audio forwarding (Android 11+).
- Quality presets: Smooth (720p/60), Balanced (1080p/60), Sharp (native/H.265).
- Connection HUD: bitrate, fps, latency indicator; graceful degradation message on bad Wi-Fi.

### Onboarding (this is where products in this space die — over-invest here)
- Guided "Enable wireless debugging" walkthrough with screenshots specific to
  Nothing OS settings paths.
- Pairing-code entry screen (6-digit) → stores pairing, never asks again.
- Health check page: same-network detection, Wi-Fi band check (warn if 2.4 GHz),
  firewall port test.

### Explicitly OUT of MVP
- iOS support, remote (non-LAN) streaming, multi-phone grid view, recording,
  virtual webcam output, phone-as-webcam. All Phase 3+ candidates.

---

## 6. Milestones

| # | Milestone | Exit criteria |
|---|---|---|
| M0 | Tech spike (1–2 wk) | Headless script: mDNS-discover Nothing Phone → adb pair → scrcpy stream over Wi-Fi at 1080p60; measure latency with on-screen stopwatch method |
| M1 | Windows MVP | Connect + mirror + control + audio on Win11, first-pair flow complete |
| M2 | Cross-platform | Same on macOS + Linux (Linux has extra adb udev friction — package rules) |
| M3 | Polish pass | Tap-to-picture ≤ 2s cold start, auto-reconnect on sleep/wake, error states designed |
| M4 | Private beta | 50 Nothing-owner testers; ≥4.5/5 smoothness rating; crash-free sessions ≥99% |

## 7. Risks

1. **Wi-Fi variance kills smoothness.** Mitigation: adaptive bitrate, band check in
   onboarding, H.265 option. Be honest in-app: "Your Wi-Fi is the bottleneck" diagnostics.
2. **Developer Options friction.** Mitigation: best-in-class guided flow; Android 11+
   pairing avoids cables entirely.
3. **OEM breakage.** Nothing OS updates may change settings paths or restrict ADB.
   Mitigation: keep device-compat matrix tested against each Nothing OS release;
   generic Android fallback keeps market open.
4. **scrcpy license:** Apache-2.0 — safe to embed; must comply with notice requirements.
5. **Firewalls/AV on Windows blocking inbound adb ports.** Mitigation: installer adds
   firewall rule with user consent; documented troubleshooting.

## 8. Design Notes (desktop app)

- Single-window app; tray icon with quick connect menu; no background daemons beyond
  the active session.
- Mirror window chrome minimal: title bar with device name, pin/fullscreen, close.
  No upsells inside the mirror window ever — trust is the product.
- Empty state (no phone found): illustration + 2 buttons ("Enable wireless debugging"
  walkthrough / "Troubleshoot"). Never a bare "No devices."
- Error states are first-class: phone locked, wrong network, firewall blocked — each
  gets a specific message + fix action, not a generic failure.

## 9. Success Metrics

- Median tap-to-picture < 2 s; p95 < 4 s.
- Session median latency ≤ 80 ms on 5 GHz Wi-Fi (measured, self-reported telemetry opt-in).
- Week-4 retention of beta users ≥ 40%.
- NPS ≥ 50 among Nothing owners.

---

## Open Questions

1. Business model: paid app (one-time), freemium (free mirror / paid control+audio), or free OSS + paid support?
2. Brand name availability ("NothingCast" likely conflicts with Nothing Technology trademarks — needs rename before launch).
3. Distribute via Microsoft Store / Setapp / direct download first?
