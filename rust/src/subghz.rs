//! Safe wrapper around the CC1101 Sub-GHz FFI for Somfy RTS transmission.
//!
//! Handles the full device lifecycle so callers don't have to wrangle unsafe
//! pointers themselves. One function to rule them all — or should we say,
//! one function to *paw*-er them all. :3

use core::ffi::c_void;
use core::ptr;

use flipperzero_sys as sys;

use crate::protocol::{self, SomfyCommand, MAX_TIMINGS};

/// Somfy RTS frequency: 433.42 MHz, the cat's meow of blind control frequencies.
const SOMFY_FREQUENCY_HZ: u32 = 433_420_000;

/// TX context passed to the yield callback via a raw pointer.
///
/// The callback is invoked from an interrupt context, so this struct
/// must live on the stack for the entire duration of the transmission.
struct TxContext {
    timings: *const sys::LevelDuration,
    count: usize,
    index: usize,
}

/// Yield callback invoked by the Sub-GHz hardware (from interrupt context).
///
/// Returns the next LevelDuration timing, or a "reset" (level=0, duration=0)
/// when all timings have been delivered — like a cat that's done knocking
/// things off the table. :3
unsafe extern "C" fn tx_yield_callback(context: *mut c_void) -> sys::LevelDuration {
    let ctx = unsafe { &mut *(context as *mut TxContext) };

    if ctx.index < ctx.count {
        let timing = unsafe { *ctx.timings.add(ctx.index) };
        ctx.index += 1;
        timing
    } else {
        // Signal end of transmission — a paws in the action
        sys::LevelDuration {
            _bitfield_align_1: [],
            _bitfield_1: sys::LevelDuration::new_bitfield_1(0, 0),
        }
    }
}

/// Transmit a Somfy RTS command over the CC1101 internal Sub-GHz radio.
///
/// This is the one safe entry point for the whole Sub-GHz dance:
/// build timings, convert formats, init hardware, transmit, clean up.
///
/// Returns `true` on success, `false` if something went wrong (device not
/// found, TX setup failed, etc.). Meow~
pub fn transmit(command: SomfyCommand, rolling_code: u16, address: u32, repeats: u8) -> bool {
    // Step 1: Build the protocol timings (pure Rust, no unsafe, purr-fect)
    let proto_timings = protocol::build_transmission(command, rolling_code, address, repeats);
    if proto_timings.is_empty() {
        return false;
    }

    // Step 2: Convert protocol::LevelDuration -> sys::LevelDuration (bitfield format)
    let mut sys_timings: heapless::Vec<sys::LevelDuration, MAX_TIMINGS> = heapless::Vec::new();
    for t in proto_timings.iter() {
        let level: u8 = if t.level { 1 } else { 0 };
        let ld = sys::LevelDuration {
            _bitfield_align_1: [],
            _bitfield_1: sys::LevelDuration::new_bitfield_1(t.duration, level),
        };
        // Should never fail since both vecs have the same capacity
        let _ = sys_timings.push(ld);
    }

    // Step 3: Set up the TX context — timings pointer stays valid on our stack
    let mut tx_ctx = TxContext {
        timings: sys_timings.as_ptr(),
        count: sys_timings.len(),
        index: 0,
    };

    // Step 4: Run the full Sub-GHz device lifecycle (the cat-alytic converter of RF)
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
                // Pass our yield callback as a raw fn pointer through c_void
                let callback_ptr = tx_yield_callback as *mut c_void;
                let context_ptr = &mut tx_ctx as *mut TxContext as *mut c_void;

                if sys::subghz_devices_start_async_tx(device, callback_ptr, context_ptr) {
                    // Poll until transmission is complete — cat nap between checks
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

    success
}
