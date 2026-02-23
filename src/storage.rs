//! Storage persistence for Somfy blind state — saves and loads from FlipperFormat files.
//!
//! Uses the same file format as the C app so state is shared between both versions.
//! Think of it as a cat-alog of your blinds, purr-sisted to disk :3

use core::ffi::{c_char, CStr};
use heapless::{String, Vec};

pub const MAX_BLINDS: usize = 8;
pub const MAX_NAME_LEN: usize = 20;

/// Path to the state file on the Flipper's SD card.
/// Matches the C app's APP_DATA_PATH("state.conf") with appid="somfy_rts".
const STATE_PATH: &CStr = c"/ext/apps_data/somfy_rts/state.conf";

/// File type header — must match the C app exactly.
const STATE_FILETYPE: &CStr = c"Somfy RTS State";

/// File format version.
const STATE_VERSION: u32 = 1;

/// A single blind's persisted state — name, address, and rolling code.
pub struct SomfyBlind {
    pub name: String<MAX_NAME_LEN>,
    pub address: u32,
    pub rolling_code: u16,
}

/// Collection of all known blinds — the whole litter, if you will :3
pub struct SomfyState {
    pub blinds: Vec<SomfyBlind, MAX_BLINDS>,
}

impl SomfyState {
    pub fn new() -> Self {
        Self {
            blinds: Vec::new(),
        }
    }
}

/// Load blind state from the FlipperFormat state file.
///
/// Returns a fresh empty state if the file doesn't exist, is corrupt, or has
/// the wrong header. Never panics — just returns what it can, like a cat
/// knocking things off a shelf and walking away.
pub fn load_state() -> SomfyState {
    let mut state = SomfyState::new();

    unsafe {
        let storage = flipperzero_sys::furi_record_open(c"storage".as_ptr())
            as *mut flipperzero_sys::Storage;
        let ff = flipperzero_sys::flipper_format_file_alloc(storage);

        'load: {
            // Open existing file
            if !flipperzero_sys::flipper_format_file_open_existing(
                ff,
                STATE_PATH.as_ptr(),
            ) {
                break 'load;
            }

            // Read and validate header
            let filetype = flipperzero_sys::furi_string_alloc();
            let mut version: u32 = 0;
            if !flipperzero_sys::flipper_format_read_header(ff, filetype, &mut version) {
                flipperzero_sys::furi_string_free(filetype);
                break 'load;
            }
            if flipperzero_sys::furi_string_cmp_str(filetype, STATE_FILETYPE.as_ptr()) != 0
                || version != STATE_VERSION
            {
                flipperzero_sys::furi_string_free(filetype);
                break 'load;
            }
            flipperzero_sys::furi_string_free(filetype);

            // Read blind count
            let mut count: u32 = 0;
            if !flipperzero_sys::flipper_format_read_uint32(ff, c"Count".as_ptr(), &mut count, 1) {
                break 'load;
            }
            if count > MAX_BLINDS as u32 {
                count = MAX_BLINDS as u32;
            }

            // Read each blind's data — one kitty at a time
            let name_str = flipperzero_sys::furi_string_alloc();
            for _ in 0..count {
                if !flipperzero_sys::flipper_format_read_string(ff, c"Name".as_ptr(), name_str) {
                    break;
                }
                let mut address: u32 = 0;
                if !flipperzero_sys::flipper_format_read_uint32(
                    ff,
                    c"Address".as_ptr(),
                    &mut address,
                    1,
                ) {
                    break;
                }
                let mut rolling_code: u32 = 0;
                if !flipperzero_sys::flipper_format_read_uint32(
                    ff,
                    c"RollingCode".as_ptr(),
                    &mut rolling_code,
                    1,
                ) {
                    break;
                }

                // Convert FuriString -> &CStr -> &str -> heapless::String
                let c_str = CStr::from_ptr(flipperzero_sys::furi_string_get_cstr(name_str));
                if let Ok(name_rust) = c_str.to_str() {
                    let mut name = String::<MAX_NAME_LEN>::new();
                    // Truncate if the name is too long — better than losing the whole blind
                    let _ = name.push_str(
                        &name_rust[..name_rust.len().min(MAX_NAME_LEN)],
                    );
                    let blind = SomfyBlind {
                        name,
                        address,
                        rolling_code: rolling_code as u16,
                    };
                    let _ = state.blinds.push(blind);
                }
            }
            flipperzero_sys::furi_string_free(name_str);
        }

        flipperzero_sys::flipper_format_free(ff);
        flipperzero_sys::furi_record_close(c"storage".as_ptr());
    }

    state
}

/// Save blind state to the FlipperFormat state file.
///
/// Returns true on success, false if something went wrong (like a cat that
/// refuses to sit where you want it to).
pub fn save_state(state: &SomfyState) -> bool {
    let mut success = false;

    unsafe {
        let storage = flipperzero_sys::furi_record_open(c"storage".as_ptr())
            as *mut flipperzero_sys::Storage;
        let ff = flipperzero_sys::flipper_format_file_alloc(storage);

        'save: {
            // Open (or create) the file — open_always truncates existing content
            if !flipperzero_sys::flipper_format_file_open_always(
                ff,
                STATE_PATH.as_ptr(),
            ) {
                break 'save;
            }

            // Write header
            if !flipperzero_sys::flipper_format_write_header_cstr(
                ff,
                STATE_FILETYPE.as_ptr(),
                STATE_VERSION,
            ) {
                break 'save;
            }

            // Write blind count
            let count: u32 = state.blinds.len() as u32;
            if !flipperzero_sys::flipper_format_write_uint32(ff, c"Count".as_ptr(), &count, 1) {
                break 'save;
            }

            // Write each blind — herding cats, but in a loop
            let mut all_ok = true;
            for blind in state.blinds.iter() {
                // Build a null-terminated name buffer.
                // heapless::String doesn't include a null terminator, so we need
                // a scratch buffer that's one byte larger — room for the \0 catnap.
                let mut name_buf = [0u8; MAX_NAME_LEN + 1];
                let name_bytes = blind.name.as_bytes();
                let len = name_bytes.len().min(MAX_NAME_LEN);
                name_buf[..len].copy_from_slice(&name_bytes[..len]);
                // name_buf[len] is already 0 from initialization

                if !flipperzero_sys::flipper_format_write_string_cstr(
                    ff,
                    c"Name".as_ptr(),
                    name_buf.as_ptr() as *const c_char,
                ) {
                    all_ok = false;
                    break;
                }

                let address = blind.address;
                if !flipperzero_sys::flipper_format_write_uint32(
                    ff,
                    c"Address".as_ptr(),
                    &address,
                    1,
                ) {
                    all_ok = false;
                    break;
                }

                let rolling_code = blind.rolling_code as u32;
                if !flipperzero_sys::flipper_format_write_uint32(
                    ff,
                    c"RollingCode".as_ptr(),
                    &rolling_code,
                    1,
                ) {
                    all_ok = false;
                    break;
                }
            }

            if !all_ok {
                break 'save;
            }

            success = true;
        }

        flipperzero_sys::flipper_format_free(ff);
        flipperzero_sys::furi_record_close(c"storage".as_ptr());
    }

    success
}
