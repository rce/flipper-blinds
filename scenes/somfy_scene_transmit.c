#include "somfy_scene.h"
#include "../somfy_protocol.h"
#include "../somfy_storage.h"

static void somfy_scene_transmit_popup_callback(void* context) {
    SomfyApp* app = context;
    scene_manager_handle_custom_event(app->scene_manager, SomfyEventTxDone);
}

void somfy_scene_transmit_on_enter(void* context) {
    SomfyApp* app = context;

    popup_reset(app->popup);
    popup_set_header(app->popup, "Sending...", 64, 20, AlignCenter, AlignCenter);
    popup_set_icon(app->popup, 0, 0, NULL);
    popup_set_text(app->popup, "Meow~", 64, 40, AlignCenter, AlignCenter);
    popup_set_timeout(app->popup, 1500);
    popup_set_callback(app->popup, somfy_scene_transmit_popup_callback);
    popup_enable_timeout(app->popup);

    view_dispatcher_switch_to_view(app->view_dispatcher, SomfyViewPopup);

    // Transmit!
    SomfyBlind* blind = &app->state.blinds[app->selected_blind];
    bool success = somfy_transmit(
        app->selected_command, blind->rolling_code, blind->address, SOMFY_TX_REPEATS);

    if(success) {
        notification_message(app->notifications, &sequence_blink_green_100);
        // Increment rolling code and save
        blind->rolling_code = (blind->rolling_code + 1) & 0xFFFF;
        somfy_state_save(&app->state);
    } else {
        notification_message(app->notifications, &sequence_blink_red_100);
    }
}

bool somfy_scene_transmit_on_event(void* context, SceneManagerEvent event) {
    SomfyApp* app = context;
    bool consumed = false;

    if(event.type == SceneManagerEventTypeCustom && event.event == SomfyEventTxDone) {
        scene_manager_search_and_switch_to_previous_scene(app->scene_manager, SomfySceneControl);
        consumed = true;
    }

    return consumed;
}

void somfy_scene_transmit_on_exit(void* context) {
    SomfyApp* app = context;
    popup_reset(app->popup);
}
