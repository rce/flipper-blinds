#include "somfy_scene.h"
#include "../somfy_storage.h"

static void somfy_scene_add_blind_text_callback(void* context) {
    SomfyApp* app = context;
    scene_manager_handle_custom_event(app->scene_manager, SomfyEventNameEntered);
}

void somfy_scene_add_blind_on_enter(void* context) {
    SomfyApp* app = context;

    text_input_reset(app->text_input);
    text_input_set_header_text(app->text_input, "Blind Name");
    text_input_set_result_callback(
        app->text_input,
        somfy_scene_add_blind_text_callback,
        app,
        app->text_input_buf,
        SOMFY_MAX_NAME_LEN,
        true); // clear default text

    // Pre-fill with "Blind"
    strncpy(app->text_input_buf, "Blind", SOMFY_MAX_NAME_LEN);

    view_dispatcher_switch_to_view(app->view_dispatcher, SomfyViewTextInput);
}

bool somfy_scene_add_blind_on_event(void* context, SceneManagerEvent event) {
    SomfyApp* app = context;
    bool consumed = false;

    if(event.type == SceneManagerEventTypeCustom && event.event == SomfyEventNameEntered) {
        // Add the new blind
        uint8_t idx = app->state.count;
        if(idx < SOMFY_MAX_BLINDS) {
            strncpy(app->state.blinds[idx].name, app->text_input_buf, SOMFY_MAX_NAME_LEN - 1);
            app->state.blinds[idx].name[SOMFY_MAX_NAME_LEN - 1] = '\0';
            app->state.blinds[idx].address = 0x100001 + idx + 1;
            app->state.blinds[idx].rolling_code = 1;
            app->state.count++;
            somfy_state_save(&app->state);
        }
        scene_manager_search_and_switch_to_previous_scene(app->scene_manager, SomfySceneMainMenu);
        consumed = true;
    }

    return consumed;
}

void somfy_scene_add_blind_on_exit(void* context) {
    SomfyApp* app = context;
    text_input_reset(app->text_input);
}
