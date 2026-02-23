# Rust Flipper Zero App Design

## Goal

Learn how to build and run a Flipper Zero app in Rust, using the existing Somfy RTS blind controller as a familiar domain for later feature work.

## Approach

Use **flipperzero-rs** (v0.16.0, the only actively maintained Rust-on-Flipper ecosystem) with the official `flipperzero-template`.

### Why flipperzero-rs

- 668+ stars, active maintenance, latest release Dec 2025
- Four-crate architecture: `flipperzero-sys` (raw FFI), `flipperzero` (safe wrappers), `flipperzero-rt` (entry point/runtime), `flipperzero-alloc` (heap via FuriMem)
- Good coverage of GUI, storage, notifications, GPIO, serial
- Sub-GHz radio has no safe bindings yet — requires manual `unsafe` FFI (good learning exercise for later)

### Constraints

- **Nightly Rust** required (for `different-binary-name` Cargo feature to produce `.fap` output)
- **Target**: `thumbv7em-none-eabihf` (STM32WB55, Cortex-M4F)
- **`no_std`** only — `core` + `alloc`, no `std`
- `.fap` binary is pinned to a Flipper SDK version — firmware updates may require recompile

## Phase 1: Hello World

Minimal app to prove the toolchain works.

### Project structure

```
rust/
├── .cargo/
│   └── config.toml          # target, linker, runner config
├── Cargo.toml                # flipperzero deps, nightly features
├── src/
│   └── main.rs               # show text on screen + blink LED + exit on Back
└── rust-toolchain.toml       # pin nightly + thumbv7em target
```

### What the app does

1. Display "Hello from Rust! :3" on the Flipper screen
2. Blink the green LED
3. Wait for Back button press
4. Exit cleanly

### Build & deploy

```bash
cd rust/
cargo build --release
# Deploy via flipperzero-tools or extend existing deploy.py
```

## Phase 2 (future): Somfy RTS features

Once Phase 1 runs on hardware, incrementally add:

1. GUI menu (blind selection) using `flipperzero::gui`
2. Storage (persist rolling codes) using `flipperzero::storage`
3. Sub-GHz transmission via `unsafe` FFI to `flipperzero-sys` (CC1101 radio API)
4. Full Somfy RTS protocol (frame building, obfuscation, Manchester encoding)

Phase 2 is intentionally vague — we'll design it once we have a working app on the device.
