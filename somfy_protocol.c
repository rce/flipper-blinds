#include "somfy_protocol.h"
#include <lib/subghz/devices/devices.h>
#include <lib/subghz/devices/cc1101_int/cc1101_int_interconnect.h>
#include <lib/subghz/devices/preset.h>

#define SOMFY_FREQUENCY 433420000

// Build plaintext 7-byte Somfy RTS frame
static void somfy_build_frame(uint8_t* frame, uint8_t command, uint16_t rolling_code, uint32_t address) {
    frame[0] = 0xA0; // Key upper nibble
    frame[1] = (command & 0x0F) << 4; // Command in upper nibble
    frame[2] = (rolling_code >> 8) & 0xFF;
    frame[3] = rolling_code & 0xFF;
    frame[4] = (address >> 16) & 0xFF;
    frame[5] = (address >> 8) & 0xFF;
    frame[6] = address & 0xFF;

    // Checksum: XOR all nibbles
    uint8_t checksum = 0;
    for(int i = 0; i < 7; i++) {
        checksum ^= (frame[i] >> 4) ^ (frame[i] & 0x0F);
    }
    frame[1] |= (checksum & 0x0F);
}

// Obfuscation: XOR each byte with previous obfuscated byte
static void somfy_obfuscate(uint8_t* frame) {
    for(int i = 1; i < 7; i++) {
        frame[i] ^= frame[i - 1];
    }
}

// Append a timing entry to the array
static inline void timing_push(LevelDuration* timings, size_t* count, size_t max, bool level, uint32_t duration) {
    if(*count < max) {
        timings[*count] = level_duration_make(level, duration);
        (*count)++;
    }
}

// Consolidate adjacent same-level entries (merge them)
static size_t timing_consolidate(LevelDuration* timings, size_t count) {
    if(count <= 1) return count;

    size_t write = 0;
    for(size_t read = 1; read < count; read++) {
        if(level_duration_get_level(timings[write]) == level_duration_get_level(timings[read])) {
            // Merge: add durations
            uint32_t merged = level_duration_get_duration(timings[write]) +
                              level_duration_get_duration(timings[read]);
            timings[write] = level_duration_make(level_duration_get_level(timings[write]), merged);
        } else {
            write++;
            timings[write] = timings[read];
        }
    }
    return write + 1;
}

size_t somfy_build_transmission(
    LevelDuration* timings,
    size_t max_timings,
    uint8_t command,
    uint16_t rolling_code,
    uint32_t address,
    uint8_t repeats) {

    // Build and obfuscate frame
    uint8_t frame[7];
    somfy_build_frame(frame, command, rolling_code, address);
    somfy_obfuscate(frame);

    size_t count = 0;

    for(uint8_t r = 0; r < repeats; r++) {
        if(r == 0) {
            // First frame: wakeup pulse + 2x hardware sync
            timing_push(timings, &count, max_timings, true, SOMFY_WAKEUP_HIGH);
            timing_push(timings, &count, max_timings, false, SOMFY_WAKEUP_LOW);
            for(int s = 0; s < 2; s++) {
                timing_push(timings, &count, max_timings, true, SOMFY_HW_SYNC_HIGH);
                timing_push(timings, &count, max_timings, false, SOMFY_HW_SYNC_LOW);
            }
        } else {
            // Repeat frames: 7x hardware sync
            for(int s = 0; s < 7; s++) {
                timing_push(timings, &count, max_timings, true, SOMFY_HW_SYNC_HIGH);
                timing_push(timings, &count, max_timings, false, SOMFY_HW_SYNC_LOW);
            }
        }

        // Software sync
        timing_push(timings, &count, max_timings, true, SOMFY_SW_SYNC_HIGH);
        timing_push(timings, &count, max_timings, false, SOMFY_SW_SYNC_LOW);

        // Manchester-encoded data
        for(int byte_idx = 0; byte_idx < 7; byte_idx++) {
            for(int bit = 7; bit >= 0; bit--) {
                bool b = (frame[byte_idx] >> bit) & 1;
                if(b) {
                    // Rising edge: low then high = bit 1
                    timing_push(timings, &count, max_timings, false, SOMFY_HALF_SYMBOL_US);
                    timing_push(timings, &count, max_timings, true, SOMFY_HALF_SYMBOL_US);
                } else {
                    // Falling edge: high then low = bit 0
                    timing_push(timings, &count, max_timings, true, SOMFY_HALF_SYMBOL_US);
                    timing_push(timings, &count, max_timings, false, SOMFY_HALF_SYMBOL_US);
                }
            }
        }

        // Inter-frame gap (except after last)
        if(r < repeats - 1) {
            timing_push(timings, &count, max_timings, false, SOMFY_INTER_FRAME_GAP);
        }
    }

    // Consolidate adjacent same-level entries
    count = timing_consolidate(timings, count);

    return count;
}

LevelDuration somfy_tx_yield(void* context) {
    SomfyTxContext* tx = context;
    if(tx->index < tx->count) {
        return tx->timings[tx->index++];
    }
    return level_duration_reset();
}

bool somfy_transmit(uint8_t command, uint16_t rolling_code, uint32_t address, uint8_t repeats) {
    // Allocate timing buffer
    LevelDuration* timings = malloc(sizeof(LevelDuration) * SOMFY_MAX_TIMINGS);
    if(!timings) return false;

    size_t count = somfy_build_transmission(timings, SOMFY_MAX_TIMINGS, command, rolling_code, address, repeats);
    if(count == 0) {
        free(timings);
        return false;
    }

    SomfyTxContext tx_ctx = {
        .timings = timings,
        .count = count,
        .index = 0,
    };

    // Init subghz devices
    subghz_devices_init();
    const SubGhzDevice* device = subghz_devices_get_by_name(SUBGHZ_DEVICE_CC1101_INT_NAME);

    bool success = false;

    if(device) {
        subghz_devices_begin(device);
        subghz_devices_load_preset(device, FuriHalSubGhzPresetOok650Async, NULL);
        subghz_devices_set_frequency(device, SOMFY_FREQUENCY);

        if(subghz_devices_set_tx(device)) {
            if(subghz_devices_start_async_tx(device, somfy_tx_yield, &tx_ctx)) {
                // Wait for TX to complete
                while(!subghz_devices_is_async_complete_tx(device)) {
                    furi_delay_ms(10);
                }
                subghz_devices_stop_async_tx(device);
                success = true;
            }
        }

        subghz_devices_idle(device);
        subghz_devices_end(device);
    }

    subghz_devices_deinit();
    free(timings);
    return success;
}
