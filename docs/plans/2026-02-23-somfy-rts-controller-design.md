# Somfy RTS Controller for Flipper Zero

## Overview

A JavaScript app for Flipper Zero that controls 2-3 Somfy RTS rolling blinds.
Handles pairing (Prog command) and basic control (Up/Down/Stop) via the
Flipper's Sub-GHz radio.

## Approach

Pure JS app — no C toolchain needed. The JS script generates Somfy RTS frames,
writes them as RAW `.sub` files, and transmits via the `subghz` module.

## Somfy RTS Protocol

Reference: https://pushstack.wordpress.com/somfy-rts-protocol/

- **Frequency**: 433.42 MHz, OOK modulation
- **Frame**: 56 bits (7 bytes), Manchester encoded, ~1208 us symbol width

### Frame layout

| Byte | Content                                          |
|------|--------------------------------------------------|
| 0    | Key: upper nibble 0xA, lower nibble = checksum   |
| 1    | Command: 0x1=Stop, 0x2=Up, 0x4=Down, 0x8=Prog   |
| 2-3  | Rolling code (16-bit, big-endian)                 |
| 4-6  | Remote address (24-bit, big-endian)               |

### Encoding steps

1. Build plaintext 7-byte frame
2. Compute checksum: XOR all nibbles of the frame, store in byte 0 lower nibble
3. Obfuscate: `frame[i] ^= frame[i-1]` for i = 1..6
4. Manchester encode: rising edge = 1, falling edge = 0, 1208 us per symbol
5. Prepend wakeup pulse + hardware sync + software sync
6. Append inter-frame gap, repeat frame 3-5 times

### Timing

- Wakeup: 9415 us high, 89565 us low (first frame only)
- Hardware sync: 2416 us high, 2416 us low (x2 first frame, x7 repeats)
- Software sync: 4550 us high, 604 us low
- Inter-frame gap: 30415 us low

## .sub File Format

```
Filetype: Flipper SubGhz RAW File
Version: 1
Frequency: 433420000
Preset: FuriHalSubGhzPresetOok650Async
Protocol: RAW
RAW_Data: <timing values in microseconds>
```

Positive values = carrier on, negative = carrier off. Up to 512 values per
`RAW_Data:` line; multiple lines allowed.

## State Persistence

JSON file at `/ext/apps_data/somfy_blinds/state.json`:

```json
{
  "blinds": [
    {"name": "Living Room", "address": 1634567, "rollingCode": 42},
    {"name": "Bedroom", "address": 1634568, "rollingCode": 17}
  ]
}
```

Each blind gets a unique 24-bit address (generated on creation). Rolling code
increments after each successful transmission and is persisted immediately.

## UI Flow

```
Main Menu              Control Menu
+-----------------+    +-----------------+
| Somfy Blinds    |    | Living Room     |
|                 |    |                 |
| > Living Room   | -> |   Up            |
|   Bedroom       |    |   Stop          |
|   + Add Blind   |    |   Down          |
+-----------------+    |   Pair          |
                       +-----------------+
```

Built with Flipper JS `submenu` module for menus and `subghz` for transmission.

## Deployment

Single file: `somfy_blinds.js` copied to `/ext/apps/Scripts/` on the Flipper
via USB serial CLI or qFlipper file manager.

## Technical Notes

- **433.42 MHz**: Not the standard 433.92. The CC1101 supports it; the 650kHz
  OOK preset bandwidth covers the offset.
- **Rolling code window**: Somfy motors accept codes within ~100 of expected
  value, so occasional missed increments are tolerable.
- **No capture needed**: We generate frames from scratch; no need to sniff an
  existing remote.
