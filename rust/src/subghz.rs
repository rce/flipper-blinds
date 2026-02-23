//! Safe wrapper around the CC1101 Sub-GHz FFI for Somfy RTS transmission.
//!
//! Uses heap allocation for timing buffers (just like the C version uses malloc),
//! because they're too large for the Flipper's limited stack. Meow~

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;
use core::ptr;

use flipperzero_sys as sys;

use crate::protocol::{self, SomfyCommand};


/// Somfy RTS frequency: 433.42 MHz
const SOMFY_FREQUENCY_HZ: u32 = 433_420_000;

/// TX context passed to the yield callback via a raw pointer.
struct TxContext {
    timings: *const sys::LevelDuration,
    count: usize,
    index: usize,
}

/// Yield callback invoked by the Sub-GHz hardware from interrupt context.
unsafe extern "C" fn tx_yield_callback(context: *mut c_void) -> sys::LevelDuration {
    let ctx = unsafe { &mut *(context as *mut TxContext) };

    if ctx.index < ctx.count {
        let timing = unsafe { *ctx.timings.add(ctx.index) };
        ctx.index += 1;
        timing
    } else {
        // Signal end of transmission
        sys::LevelDuration {
            _bitfield_align_1: [],
            _bitfield_1: sys::LevelDuration::new_bitfield_1(0, 0),
        }
    }
}

/// Transmit a Somfy RTS command over the CC1101 internal Sub-GHz radio.
///
/// Returns `true` on success, `false` on failure.
pub fn transmit(command: SomfyCommand, rolling_code: u16, address: u32, repeats: u8) -> bool {
    // Build protocol timings (pure Rust, on stack — heapless::Vec is fine here)
    let proto_timings = protocol::build_transmission(command, rolling_code, address, repeats);
    if proto_timings.is_empty() {
        return false;
    }

    // Convert to sys::LevelDuration on the HEAP (too large for stack)
    let sys_timings: Vec<sys::LevelDuration> = proto_timings
        .iter()
        .map(|t| {
            let level: u8 = if t.level { 1 } else { 0 };
            sys::LevelDuration {
                _bitfield_align_1: [],
                _bitfield_1: sys::LevelDuration::new_bitfield_1(t.duration, level),
            }
        })
        .collect();

    let mut tx_ctx = TxContext {
        timings: sys_timings.as_ptr(),
        count: sys_timings.len(),
        index: 0,
    };

    let mut success = false;

    unsafe {
        sys::subghz_devices_init();

        let device = sys::subghz_devices_get_by_name(c"cc1101_int".as_ptr());

        if !device.is_null() {
            sys::subghz_devices_begin(device);
            sys::subghz_devices_load_preset(
                device,
                sys::FuriHalSubGhzPresetOok650Async,
                ptr::null_mut(),
            );
            sys::subghz_devices_set_frequency(device, SOMFY_FREQUENCY_HZ);

            if sys::subghz_devices_set_tx(device) {
                let callback_ptr = tx_yield_callback as *mut c_void;
                let context_ptr = &mut tx_ctx as *mut TxContext as *mut c_void;

                if sys::subghz_devices_start_async_tx(device, callback_ptr, context_ptr) {
                    while !sys::subghz_devices_is_async_complete_tx(device) {
                        sys::furi_delay_ms(10);
                    }
                    sys::subghz_devices_stop_async_tx(device);
                    success = true;
                }
            }

            sys::subghz_devices_idle(device);
            sys::subghz_devices_end(device);
        }

        sys::subghz_devices_deinit();
    }

    // sys_timings dropped here — after TX is complete
    success
}
