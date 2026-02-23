#include "somfy_scene.h"
#include "../somfy_protocol.h"
#include "../somfy_storage.h"

void somfy_scene_transmit_on_enter(void* context) {
    SomfyApp* app = context;

    popup_reset(app->popup);
    popup_set_header(app->popup, "Sending...", 64, 20, AlignCenter, AlignCenter);
    popup_set_text(app->popup, "Meow~", 64, 40, AlignCenter, AlignCenter);

    view_dispatcher_switch_to_view(app->view_dispatcher, SomfyViewPopup);

    // Transmit (blocks briefly ~500ms for 4 repeats)
    SomfyBlind* blind = &app->state.blinds[app->selected_blind];
    bool success = somfy_transmit(
        app->selected_command, blind->rolling_code, blind->address, SOMFY_TX_REPEATS);

    if(success) {
        notification_message(app->notifications, &sequence_blink_green_100);
        blind->rolling_code = (blind->rolling_code + 1) & 0xFFFF;
        somfy_state_save(&app->state);
    } else {
        notification_message(app->notifications, &sequence_blink_red_100);
    }

    // Navigate back after TX — event is queued and processed after on_enter returns
    view_dispatcher_send_custom_event(app->view_dispatcher, SomfyEventTxDone);
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
