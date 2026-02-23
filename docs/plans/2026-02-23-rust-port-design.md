# Rust Port of Somfy RTS Controller — Design

## Goal

Port the C Flipper Zero Somfy RTS blind controller to Rust, bottom-up: pure protocol logic first, then safe wrappers around unsafe FFI for Sub-GHz and storage, finally a dialog-based UI.

## Architecture

Incremental, bottom-up port using flipperzero-rs v0.16.0. Pure safe Rust for protocol logic, unsafe FFI quarantined behind safe wrapper modules. Dialog-based UI (hybrid approach — simpler than C's SceneManager, can evolve later).

## Modules

### `protocol.rs` — Pure Rust, no unsafe
- Frame building: `build_frame(cmd, rolling_code, address) -> [u8; 7]`
- Checksum: XOR all 8 nibbles
- Obfuscation: rolling XOR cipher
- Manchester encoding: bits to (level, duration_us) pairs
- Full transmission builder with wakeup/sync/repeats/consolidation
- Uses `heapless` for fixed-capacity no_std collections

### `subghz.rs` — Safe wrapper around CC1101 FFI
- Public API: `transmit(timings: &[LevelDuration]) -> bool`
- Internally: init CC1101, load OOK650Async preset, set 433.42 MHz, async TX with yield callback, poll completion, cleanup
- All unsafe quarantined inside module

### `storage.rs` — Safe wrapper around FlipperFormat FFI
- `load_state() -> SomfyState` (empty state if file missing)
- `save_state(state: &SomfyState) -> bool`
- Same file format and path as C app (`/ext/apps_data/somfy_rts/state.conf`)
- Cross-compatible with C app state

### `main.rs` — Dialog-based UI
- Load state on startup
- Blind selection via Prev/Select/Next dialog buttons
- Control via Up/Stop/Down dialog buttons
- Additional options dialog for Pair/Remove
- Auto-naming for new blinds ("Blind 1", "Blind 2", etc.)
- LED feedback: green=success, red=fail

## Shared Types

```rust
pub enum SomfyCommand { Stop = 0x1, Up = 0x2, Down = 0x4, Prog = 0x8 }

pub struct SomfyBlind {
    pub name: heapless::String<20>,
    pub address: u32,      // 24-bit
    pub rolling_code: u16,
}

pub struct SomfyState {
    pub blinds: heapless::Vec<SomfyBlind, 8>,
}
```

## Constants (matching C app exactly)
- Frequency: 433,420,000 Hz
- Symbol: 1208µs, half-symbol: 604µs
- Wakeup: 9415µs / 89565µs
- HW sync: 2416µs / 2416µs, SW sync: 4550µs / 604µs
- Inter-frame gap: 30415µs
- TX repeats: 4
- Max blinds: 8, max name length: 20

## Cross-compatibility
- Same storage format and path as C app
- Same address generation: 0x100001 + index + 1
- Same protocol, same RF timings
- Rust app can pick up C app's state and vice versa
