#include "somfy_storage.h"
#include <lib/flipper_format/flipper_format.h>
#include <storage/storage.h>

#define SOMFY_STATE_PATH APP_DATA_PATH("state.conf")
#define SOMFY_STATE_FILETYPE "Somfy RTS State"
#define SOMFY_STATE_VERSION 1

void somfy_state_load(SomfyState* state) {
    state->count = 0;

    Storage* storage = furi_record_open(RECORD_STORAGE);
    FlipperFormat* ff = flipper_format_file_alloc(storage);

    do {
        if(!flipper_format_file_open_existing(ff, SOMFY_STATE_PATH)) break;

        FuriString* filetype = furi_string_alloc();
        uint32_t version = 0;
        if(!flipper_format_read_header(ff, filetype, &version)) {
            furi_string_free(filetype);
            break;
        }
        if(furi_string_cmp_str(filetype, SOMFY_STATE_FILETYPE) != 0 || version != SOMFY_STATE_VERSION) {
            furi_string_free(filetype);
            break;
        }
        furi_string_free(filetype);

        uint32_t count = 0;
        if(!flipper_format_read_uint32(ff, "Count", &count, 1)) break;
        if(count > SOMFY_MAX_BLINDS) count = SOMFY_MAX_BLINDS;

        FuriString* name_str = furi_string_alloc();
        for(uint32_t i = 0; i < count; i++) {
            if(!flipper_format_read_string(ff, "Name", name_str)) break;
            uint32_t address = 0;
            if(!flipper_format_read_uint32(ff, "Address", &address, 1)) break;
            uint32_t rolling_code = 0;
            if(!flipper_format_read_uint32(ff, "RollingCode", &rolling_code, 1)) break;

            strncpy(state->blinds[i].name, furi_string_get_cstr(name_str), SOMFY_MAX_NAME_LEN - 1);
            state->blinds[i].name[SOMFY_MAX_NAME_LEN - 1] = '\0';
            state->blinds[i].address = address;
            state->blinds[i].rolling_code = (uint16_t)(rolling_code & 0xFFFF);
            state->count++;
        }
        furi_string_free(name_str);
    } while(false);

    flipper_format_free(ff);
    furi_record_close(RECORD_STORAGE);
}

bool somfy_state_save(const SomfyState* state) {
    Storage* storage = furi_record_open(RECORD_STORAGE);
    FlipperFormat* ff = flipper_format_file_alloc(storage);
    bool success = false;

    do {
        if(!flipper_format_file_open_always(ff, SOMFY_STATE_PATH)) break;

        if(!flipper_format_write_header_cstr(ff, SOMFY_STATE_FILETYPE, SOMFY_STATE_VERSION)) break;

        uint32_t count = state->count;
        if(!flipper_format_write_uint32(ff, "Count", &count, 1)) break;

        bool all_ok = true;
        for(uint8_t i = 0; i < state->count; i++) {
            if(!flipper_format_write_string_cstr(ff, "Name", state->blinds[i].name)) {
                all_ok = false;
                break;
            }
            uint32_t address = state->blinds[i].address;
            if(!flipper_format_write_uint32(ff, "Address", &address, 1)) {
                all_ok = false;
                break;
            }
            uint32_t rolling_code = state->blinds[i].rolling_code;
            if(!flipper_format_write_uint32(ff, "RollingCode", &rolling_code, 1)) {
                all_ok = false;
                break;
            }
        }
        if(!all_ok) break;

        success = true;
    } while(false);

    flipper_format_free(ff);
    furi_record_close(RECORD_STORAGE);
    return success;
}
