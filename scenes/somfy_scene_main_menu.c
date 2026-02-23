#include "somfy_scene.h"

#define ADD_BLIND_INDEX 0xFF

static void somfy_scene_main_menu_callback(void* context, uint32_t index) {
    SomfyApp* app = context;
    if(index == ADD_BLIND_INDEX) {
        scene_manager_handle_custom_event(app->scene_manager, SomfyEventAddBlind);
    } else {
        app->selected_blind = index;
        scene_manager_handle_custom_event(app->scene_manager, SomfyEventBlindSelected);
    }
}

void somfy_scene_main_menu_on_enter(void* context) {
    SomfyApp* app = context;
    submenu_reset(app->submenu);
    submenu_set_header(app->submenu, "Somfy Blinds");

    for(uint8_t i = 0; i < app->state.count; i++) {
        submenu_add_item(app->submenu, app->state.blinds[i].name, i, somfy_scene_main_menu_callback, app);
    }

    if(app->state.count < SOMFY_MAX_BLINDS) {
        submenu_add_item(app->submenu, "+ Add Blind", ADD_BLIND_INDEX, somfy_scene_main_menu_callback, app);
    }

    view_dispatcher_switch_to_view(app->view_dispatcher, SomfyViewSubmenu);
}

bool somfy_scene_main_menu_on_event(void* context, SceneManagerEvent event) {
    SomfyApp* app = context;
    bool consumed = false;

    if(event.type == SceneManagerEventTypeCustom) {
        if(event.event == SomfyEventBlindSelected) {
            scene_manager_next_scene(app->scene_manager, SomfySceneControl);
            consumed = true;
        } else if(event.event == SomfyEventAddBlind) {
            scene_manager_next_scene(app->scene_manager, SomfySceneAddBlind);
            consumed = true;
        }
    }

    return consumed;
}

void somfy_scene_main_menu_on_exit(void* context) {
    SomfyApp* app = context;
    submenu_reset(app->submenu);
}
