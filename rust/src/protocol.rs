//! Somfy RTS protocol implementation — pure Rust, no unsafe, no flipperzero imports.
//!
//! Purr-fectly encodes frames for controlling Somfy blinds via the RTS protocol.
//! Manchester encoding, obfuscation, and transmission building — the whole kitten caboodle.

use heapless::Vec;

// Timing constants (in microseconds) — must match the C implementation exactly
const SOMFY_HALF_SYMBOL_US: u32 = 604;
const SOMFY_WAKEUP_HIGH: u32 = 9415;
const SOMFY_WAKEUP_LOW: u32 = 89565;
const SOMFY_HW_SYNC_HIGH: u32 = 2416;
const SOMFY_HW_SYNC_LOW: u32 = 2416;
const SOMFY_SW_SYNC_HIGH: u32 = 4550;
const SOMFY_SW_SYNC_LOW: u32 = 604;
const SOMFY_INTER_FRAME_GAP: u32 = 30415;

pub const MAX_TIMINGS: usize = 600;

/// A level (high/low) and duration pair for sub-GHz transmission.
///
/// We define our own struct here rather than using the flipperzero-sys bitfield one.
/// The subghz module will handle conversion between the two.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelDuration {
    pub level: bool,
    pub duration: u32,
}

/// Somfy RTS command nibbles — each fits in the upper nibble of frame byte 1.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SomfyCommand {
    Stop = 0x1,
    Up = 0x2,
    Down = 0x4,
    Prog = 0x8,
}

/// Build a 7-byte Somfy RTS frame with the given command, rolling code, and address.
///
/// The frame layout is:
///   [0] = 0xA0 (key/encryption byte)
///   [1] = (command << 4) | checksum
///   [2..3] = rolling code (big-endian)
///   [4..6] = address (big-endian, 24-bit)
///
/// The checksum is the XOR of all nibbles (upper ^ lower) across all 7 bytes.
pub fn build_frame(command: SomfyCommand, rolling_code: u16, address: u32) -> [u8; 7] {
    let mut frame = [0u8; 7];

    frame[0] = 0xA0;
    frame[1] = (command as u8 & 0x0F) << 4;
    frame[2] = (rolling_code >> 8) as u8;
    frame[3] = rolling_code as u8;
    frame[4] = (address >> 16) as u8;
    frame[5] = (address >> 8) as u8;
    frame[6] = address as u8;

    // Checksum: XOR of all nibbles
    let mut checksum: u8 = 0;
    for byte in &frame {
        checksum ^= (byte >> 4) ^ (byte & 0x0F);
    }
    frame[1] |= checksum & 0x0F;

    frame
}

/// Obfuscate a frame by XOR-ing each byte with the previous one.
///
/// This is the Somfy RTS "encryption" — each byte[i] ^= byte[i-1], starting from index 1.
/// Must be applied after build_frame and before transmission.
pub fn obfuscate(frame: &mut [u8; 7]) {
    for i in 1..7 {
        frame[i] ^= frame[i - 1];
    }
}

/// Build a complete Somfy RTS transmission as a sequence of level/duration pairs.
///
/// This handles:
/// - Wakeup pulse (first frame only) + HW sync
/// - SW sync
/// - Manchester-encoded data (56 bits, MSB first)
/// - Inter-frame gaps between repeats
/// - Consolidation of adjacent same-level entries (meow-rging them together)
pub fn build_transmission(
    command: SomfyCommand,
    rolling_code: u16,
    address: u32,
    repeats: u8,
) -> Vec<LevelDuration, MAX_TIMINGS> {
    let mut frame = build_frame(command, rolling_code, address);
    obfuscate(&mut frame);

    // First pass: build raw (unconsolidated) timings
    let mut raw: Vec<LevelDuration, MAX_TIMINGS> = Vec::new();

    for r in 0..repeats {
        if r == 0 {
            // Wakeup pulse
            let _ = raw.push(LevelDuration { level: true, duration: SOMFY_WAKEUP_HIGH });
            let _ = raw.push(LevelDuration { level: false, duration: SOMFY_WAKEUP_LOW });
            // 2x HW sync
            for _ in 0..2 {
                let _ = raw.push(LevelDuration { level: true, duration: SOMFY_HW_SYNC_HIGH });
                let _ = raw.push(LevelDuration { level: false, duration: SOMFY_HW_SYNC_LOW });
            }
        } else {
            // 7x HW sync
            for _ in 0..7 {
                let _ = raw.push(LevelDuration { level: true, duration: SOMFY_HW_SYNC_HIGH });
                let _ = raw.push(LevelDuration { level: false, duration: SOMFY_HW_SYNC_LOW });
            }
        }

        // SW sync
        let _ = raw.push(LevelDuration { level: true, duration: SOMFY_SW_SYNC_HIGH });
        let _ = raw.push(LevelDuration { level: false, duration: SOMFY_SW_SYNC_LOW });

        // Manchester-encode 56 bits (7 bytes, MSB first)
        // Bit 1 = rising edge: low then high
        // Bit 0 = falling edge: high then low
        for byte in &frame {
            for bit_pos in (0..8).rev() {
                let bit = (byte >> bit_pos) & 1;
                if bit == 1 {
                    let _ = raw.push(LevelDuration { level: false, duration: SOMFY_HALF_SYMBOL_US });
                    let _ = raw.push(LevelDuration { level: true, duration: SOMFY_HALF_SYMBOL_US });
                } else {
                    let _ = raw.push(LevelDuration { level: true, duration: SOMFY_HALF_SYMBOL_US });
                    let _ = raw.push(LevelDuration { level: false, duration: SOMFY_HALF_SYMBOL_US });
                }
            }
        }

        // Inter-frame gap (except after last frame)
        if r + 1 < repeats {
            let _ = raw.push(LevelDuration { level: false, duration: SOMFY_INTER_FRAME_GAP });
        }
    }

    // Consolidation: merge adjacent entries with the same level
    let mut consolidated: Vec<LevelDuration, MAX_TIMINGS> = Vec::new();

    for entry in raw.iter() {
        if let Some(last) = consolidated.last_mut() {
            if last.level == entry.level {
                last.duration += entry.duration;
                continue;
            }
        }
        let _ = consolidated.push(*entry);
    }

    consolidated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_frame_checksum() {
        // Verify frame structure and that checksum nibble is correctly computed
        let frame = build_frame(SomfyCommand::Up, 0x0042, 0xCAFE00);
        assert_eq!(frame[0], 0xA0, "key byte should be 0xA0");
        assert_eq!(frame[1] >> 4, SomfyCommand::Up as u8, "command nibble");
        assert_eq!(frame[2], 0x00, "rolling code high byte");
        assert_eq!(frame[3], 0x42, "rolling code low byte");
        assert_eq!(frame[4], 0xCA, "address byte 2");
        assert_eq!(frame[5], 0xFE, "address byte 1");
        assert_eq!(frame[6], 0x00, "address byte 0");

        // Verify checksum: XOR of all nibbles should be 0 (it's self-consistent)
        let mut check: u8 = 0;
        for byte in &frame {
            check ^= (byte >> 4) ^ (byte & 0x0F);
        }
        assert_eq!(check, 0, "checksum should make all nibbles XOR to 0");
    }

    #[test]
    fn test_obfuscate() {
        let mut frame = build_frame(SomfyCommand::Stop, 0x0001, 0x123456);
        let original = frame;
        obfuscate(&mut frame);

        // Byte 0 should be unchanged
        assert_eq!(frame[0], original[0]);
        // Each subsequent byte should be XOR'd with the previous (obfuscated) byte
        for i in 1..7 {
            assert_eq!(frame[i], original[i] ^ frame[i - 1]);
        }
    }

    #[test]
    fn test_build_transmission_not_empty() {
        let timings = build_transmission(SomfyCommand::Up, 1, 0xABCDEF, 3);
        assert!(!timings.is_empty(), "transmission should produce timings");
    }

    #[test]
    fn test_build_transmission_starts_with_wakeup() {
        let timings = build_transmission(SomfyCommand::Down, 42, 0x112233, 1);
        // First entry should be the wakeup high pulse
        assert_eq!(timings[0].level, true);
        assert_eq!(timings[0].duration, SOMFY_WAKEUP_HIGH);
    }

    #[test]
    fn test_consolidation_merges_adjacent_levels() {
        let timings = build_transmission(SomfyCommand::Up, 1, 0x000001, 1);
        // After consolidation, no two adjacent entries should have the same level
        for window in timings.windows(2) {
            assert_ne!(
                window[0].level, window[1].level,
                "adjacent entries must alternate levels after consolidation"
            );
        }
    }

    #[test]
    fn test_single_frame_no_trailing_gap() {
        let timings = build_transmission(SomfyCommand::Prog, 99, 0xFEDCBA, 1);
        // Last entry should NOT be the inter-frame gap (only added between frames)
        let last = timings.last().unwrap();
        // With a single repeat, the last timing comes from manchester data, not a gap
        assert!(last.duration != SOMFY_INTER_FRAME_GAP || last.level == true);
    }

    #[test]
    fn test_all_commands() {
        // Just make sure all command variants produce valid frames — no panics, nyaa~
        for cmd in [SomfyCommand::Stop, SomfyCommand::Up, SomfyCommand::Down, SomfyCommand::Prog] {
            let frame = build_frame(cmd, 0, 0);
            assert_eq!(frame[1] >> 4, cmd as u8);
        }
    }
}
