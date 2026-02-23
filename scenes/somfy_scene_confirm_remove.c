#include "somfy_scene.h"
#include "../somfy_storage.h"

static void somfy_scene_confirm_remove_callback(DialogExResult result, void* context) {
    SomfyApp* app = context;
    if(result == DialogExResultRight) {
        scene_manager_handle_custom_event(app->scene_manager, SomfyEventRemoveConfirmed);
    } else {
        scene_manager_handle_back_event(app->scene_manager);
    }
}

void somfy_scene_confirm_remove_on_enter(void* context) {
    SomfyApp* app = context;
    SomfyBlind* blind = &app->state.blinds[app->selected_blind];

    dialog_ex_reset(app->dialog_ex);
    dialog_ex_set_header(app->dialog_ex, "Remove blind?", 64, 0, AlignCenter, AlignTop);
    dialog_ex_set_text(app->dialog_ex, blind->name, 64, 32, AlignCenter, AlignCenter);
    dialog_ex_set_left_button_text(app->dialog_ex, "Cancel");
    dialog_ex_set_right_button_text(app->dialog_ex, "Remove");
    dialog_ex_set_result_callback(app->dialog_ex, somfy_scene_confirm_remove_callback);
    dialog_ex_set_context(app->dialog_ex, app);

    view_dispatcher_switch_to_view(app->view_dispatcher, SomfyViewDialogEx);
}

bool somfy_scene_confirm_remove_on_event(void* context, SceneManagerEvent event) {
    SomfyApp* app = context;
    bool consumed = false;

    if(event.type == SceneManagerEventTypeCustom && event.event == SomfyEventRemoveConfirmed) {
        // Shift remaining blinds down
        uint8_t idx = app->selected_blind;
        for(uint8_t i = idx; i < app->state.count - 1; i++) {
            app->state.blinds[i] = app->state.blinds[i + 1];
        }
        app->state.count--;
        somfy_state_save(&app->state);

        // Go back to main menu
        scene_manager_search_and_switch_to_previous_scene(app->scene_manager, SomfySceneMainMenu);
        consumed = true;
    }

    return consumed;
}

void somfy_scene_confirm_remove_on_exit(void* context) {
    SomfyApp* app = context;
    dialog_ex_reset(app->dialog_ex);
}
