// Somfy RTS Blind Controller for Flipper Zero
print("Somfy Blinds starting, nyaa~");

// Command constants
let CMD_STOP = 0x1;
let CMD_UP   = 0x2;
let CMD_DOWN = 0x4;
let CMD_PROG = 0x8;

// Build plaintext 7-byte Somfy RTS frame
function buildFrame(command, rollingCode, address) {
    let frame = [0, 0, 0, 0, 0, 0, 0];

    // Byte 0: key (upper nibble 0xA, lower nibble filled with checksum later)
    frame[0] = 0xA0;

    // Byte 1: command in upper nibble
    frame[1] = (command & 0x0F) << 4;

    // Bytes 2-3: rolling code (big-endian)
    frame[2] = (rollingCode >> 8) & 0xFF;
    frame[3] = rollingCode & 0xFF;

    // Bytes 4-6: address (big-endian)
    frame[4] = (address >> 16) & 0xFF;
    frame[5] = (address >> 8) & 0xFF;
    frame[6] = address & 0xFF;

    // Checksum: XOR all nibbles
    let checksum = 0;
    for (let i = 0; i < 7; i++) {
        checksum = checksum ^ (frame[i] >> 4) ^ (frame[i] & 0x0F);
    }
    frame[1] = frame[1] | (checksum & 0x0F);

    return frame;
}

// Obfuscation: XOR each byte with previous obfuscated byte
function obfuscate(frame) {
    let result = [frame[0]];
    for (let i = 1; i < 7; i++) {
        result.push(frame[i] ^ result[i - 1]);
    }
    return result;
}

// Manchester encoding constants
let SYMBOL_US = 1208;
let HALF_SYMBOL_US = 604;

// Manchester encode: rising edge (low->high) = 1, falling edge (high->low) = 0
function manchesterEncode(frame) {
    let timings = [];
    for (let i = 0; i < 7; i++) {
        for (let bit = 7; bit >= 0; bit--) {
            let b = (frame[i] >> bit) & 1;
            if (b === 1) {
                // Rising edge: low then high
                timings.push(-HALF_SYMBOL_US);
                timings.push(HALF_SYMBOL_US);
            } else {
                // Falling edge: high then low
                timings.push(HALF_SYMBOL_US);
                timings.push(-HALF_SYMBOL_US);
            }
        }
    }
    return timings;
}

// Transmission timing constants
let WAKEUP_HIGH = 9415;
let WAKEUP_LOW = 89565;
let HW_SYNC_HIGH = 2416;
let HW_SYNC_LOW = 2416;
let SW_SYNC_HIGH = 4550;
let SW_SYNC_LOW = 604;
let INTER_FRAME_GAP = 30415;

// Build full transmission with sync pulses and repeats
function buildTransmission(command, rollingCode, address, repeats) {
    let frame = buildFrame(command, rollingCode, address);
    let obfuscated = obfuscate(frame);
    let manchester = manchesterEncode(obfuscated);

    let timings = [];

    for (let r = 0; r < repeats; r++) {
        if (r === 0) {
            // First frame: wakeup + 2x hardware sync
            timings.push(WAKEUP_HIGH);
            timings.push(-WAKEUP_LOW);
            for (let s = 0; s < 2; s++) {
                timings.push(HW_SYNC_HIGH);
                timings.push(-HW_SYNC_LOW);
            }
        } else {
            // Repeat frames: 7x hardware sync
            for (let s = 0; s < 7; s++) {
                timings.push(HW_SYNC_HIGH);
                timings.push(-HW_SYNC_LOW);
            }
        }

        // Software sync
        timings.push(SW_SYNC_HIGH);
        timings.push(-SW_SYNC_LOW);

        // Manchester-encoded data
        for (let t = 0; t < manchester.length; t++) {
            timings.push(manchester[t]);
        }

        // Inter-frame gap (except after last repeat)
        if (r < repeats - 1) {
            timings.push(-INTER_FRAME_GAP);
        }
    }

    return timings;
}

// Self-test: print a frame for known values
let testFrame = buildFrame(CMD_UP, 1, 0x123456);
let testOb = obfuscate(testFrame);
print("Frame: " + testFrame.join(","));
print("Obfuscated: " + testOb.join(","));
