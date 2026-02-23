#pragma once

#include <furi.h>
#include <lib/toolbox/level_duration.h>

// Somfy RTS timing constants (microseconds)
#define SOMFY_SYMBOL_US 1208
#define SOMFY_HALF_SYMBOL_US 604
#define SOMFY_WAKEUP_HIGH 9415
#define SOMFY_WAKEUP_LOW 89565
#define SOMFY_HW_SYNC_HIGH 2416
#define SOMFY_HW_SYNC_LOW 2416
#define SOMFY_SW_SYNC_HIGH 4550
#define SOMFY_SW_SYNC_LOW 604
#define SOMFY_INTER_FRAME_GAP 30415

// Max timings per transmission (generous upper bound)
// Per frame: wakeup(2) + hw_sync(14) + sw_sync(2) + manchester(112) + gap(1) = ~131
// 4 repeats: ~524, round up
#define SOMFY_MAX_TIMINGS 600

// Transmission context for async TX callback
typedef struct {
    LevelDuration* timings;
    size_t count;
    size_t index;
} SomfyTxContext;

// Build a complete Somfy RTS transmission into a LevelDuration array.
// Returns number of LevelDuration entries written.
size_t somfy_build_transmission(
    LevelDuration* timings,
    size_t max_timings,
    uint8_t command,
    uint16_t rolling_code,
    uint32_t address,
    uint8_t repeats);

// Async TX yield callback — pass SomfyTxContext* as context
LevelDuration somfy_tx_yield(void* context);

// High-level: transmit a Somfy RTS command. Blocks until TX complete.
// Returns true on success.
bool somfy_transmit(uint8_t command, uint16_t rolling_code, uint32_t address, uint8_t repeats);
