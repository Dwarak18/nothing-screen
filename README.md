# nothing-screen

Wireless screen mirroring from a Nothing Phone to a desktop, with scrcpy-class
smoothness and AirPlay-class ease-of-use.

## What this is

A Tauri 2 + Rust desktop shell (Windows / Linux / macOS) with a Vite +
TypeScript UI. It discovers Android phones on the LAN via mDNS (Wireless
debugging), pairs over `adb`, and mirrors the screen using the scrcpy engine.

## Stack
- **Desktop shell:** Tauri 2 + Rust (Windows focus, cross-platform ready)
- **UI:** Vite + TypeScript
- **Engine:** `adb` + `scrcpy` client (`src-tauri/binaries/`)
- **Discovery:** `mdns-sd` crate (`_adb-tls-connect` / `_adb-tls-pairing`)

## Run (Windows dev)
```
npm install
npm run tauri dev
```

## Conventions
- Rust modules in `src-tauri/src/`; each maps to one concern:
  `adb.rs` (transport), `mdns.rs` (discovery), `scrcpy.rs` (streaming),
  `state.rs` (shared state), `lib.rs` (Tauri commands + event pump).
- Plan: `PLAN.md` · Execution: `TASKS.md`.