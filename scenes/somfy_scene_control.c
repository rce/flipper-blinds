#include "somfy_scene.h"

typedef enum {
    ControlIndexUp,
    ControlIndexStop,
    ControlIndexDown,
    ControlIndexPair,
    ControlIndexRemove,
} ControlIndex;

static void somfy_scene_control_callback(void* context, uint32_t index) {
    SomfyApp* app = context;
    if(index == ControlIndexRemove) {
        scene_manager_handle_custom_event(app->scene_manager, SomfyEventRemoveBlind);
        return;
    }
    switch(index) {
    case ControlIndexUp:
        app->selected_command = SomfyCmdUp;
        break;
    case ControlIndexStop:
        app->selected_command = SomfyCmdStop;
        break;
    case ControlIndexDown:
        app->selected_command = SomfyCmdDown;
        break;
    case ControlIndexPair:
        app->selected_command = SomfyCmdProg;
        break;
    default:
        break;
    }
    scene_manager_handle_custom_event(app->scene_manager, SomfyEventCommandSelected);
}

void somfy_scene_control_on_enter(void* context) {
    SomfyApp* app = context;
    submenu_reset(app->submenu);

    SomfyBlind* blind = &app->state.blinds[app->selected_blind];
    submenu_set_header(app->submenu, blind->name);

    submenu_add_item(app->submenu, "Up", ControlIndexUp, somfy_scene_control_callback, app);
    submenu_add_item(app->submenu, "Stop", ControlIndexStop, somfy_scene_control_callback, app);
    submenu_add_item(app->submenu, "Down", ControlIndexDown, somfy_scene_control_callback, app);
    submenu_add_item(app->submenu, "Pair (Prog)", ControlIndexPair, somfy_scene_control_callback, app);
    submenu_add_item(app->submenu, "Remove", ControlIndexRemove, somfy_scene_control_callback, app);

    view_dispatcher_switch_to_view(app->view_dispatcher, SomfyViewSubmenu);
}

bool somfy_scene_control_on_event(void* context, SceneManagerEvent event) {
    SomfyApp* app = context;
    bool consumed = false;

    if(event.type == SceneManagerEventTypeCustom) {
        if(event.event == SomfyEventCommandSelected) {
            scene_manager_next_scene(app->scene_manager, SomfySceneTransmit);
            consumed = true;
        } else if(event.event == SomfyEventRemoveBlind) {
            scene_manager_next_scene(app->scene_manager, SomfySceneConfirmRemove);
            consumed = true;
        }
    }

    return consumed;
}

void somfy_scene_control_on_exit(void* context) {
    SomfyApp* app = context;
    submenu_reset(app->submenu);
}
