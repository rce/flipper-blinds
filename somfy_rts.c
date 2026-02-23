#include "somfy_rts.h"
#include "somfy_storage.h"
#include "scenes/somfy_scene.h"

// Scene handler tables
static void (*const somfy_scene_on_enter_handlers[])(void*) = {
    somfy_scene_main_menu_on_enter,
    somfy_scene_control_on_enter,
    somfy_scene_add_blind_on_enter,
    somfy_scene_transmit_on_enter,
    somfy_scene_confirm_remove_on_enter,
};

static bool (*const somfy_scene_on_event_handlers[])(void*, SceneManagerEvent) = {
    somfy_scene_main_menu_on_event,
    somfy_scene_control_on_event,
    somfy_scene_add_blind_on_event,
    somfy_scene_transmit_on_event,
    somfy_scene_confirm_remove_on_event,
};

static void (*const somfy_scene_on_exit_handlers[])(void*) = {
    somfy_scene_main_menu_on_exit,
    somfy_scene_control_on_exit,
    somfy_scene_add_blind_on_exit,
    somfy_scene_transmit_on_exit,
    somfy_scene_confirm_remove_on_exit,
};

static const SceneManagerHandlers somfy_scene_handlers = {
    .on_enter_handlers = somfy_scene_on_enter_handlers,
    .on_event_handlers = somfy_scene_on_event_handlers,
    .on_exit_handlers = somfy_scene_on_exit_handlers,
    .scene_num = SomfySceneCount,
};

static bool somfy_custom_event_callback(void* context, uint32_t event) {
    furi_assert(context);
    SomfyApp* app = context;
    return scene_manager_handle_custom_event(app->scene_manager, event);
}

static bool somfy_back_event_callback(void* context) {
    furi_assert(context);
    SomfyApp* app = context;
    return scene_manager_handle_back_event(app->scene_manager);
}

static SomfyApp* somfy_app_alloc(void) {
    SomfyApp* app = malloc(sizeof(SomfyApp));

    app->gui = furi_record_open(RECORD_GUI);
    app->notifications = furi_record_open(RECORD_NOTIFICATION);

    app->scene_manager = scene_manager_alloc(&somfy_scene_handlers, app);
    app->view_dispatcher = view_dispatcher_alloc();
    view_dispatcher_set_event_callback_context(app->view_dispatcher, app);
    view_dispatcher_set_custom_event_callback(app->view_dispatcher, somfy_custom_event_callback);
    view_dispatcher_set_navigation_event_callback(app->view_dispatcher, somfy_back_event_callback);
    view_dispatcher_attach_to_gui(app->view_dispatcher, app->gui, ViewDispatcherTypeFullscreen);

    // Allocate views
    app->submenu = submenu_alloc();
    view_dispatcher_add_view(
        app->view_dispatcher, SomfyViewSubmenu, submenu_get_view(app->submenu));

    app->text_input = text_input_alloc();
    view_dispatcher_add_view(
        app->view_dispatcher, SomfyViewTextInput, text_input_get_view(app->text_input));

    app->popup = popup_alloc();
    view_dispatcher_add_view(
        app->view_dispatcher, SomfyViewPopup, popup_get_view(app->popup));

    app->dialog_ex = dialog_ex_alloc();
    view_dispatcher_add_view(
        app->view_dispatcher, SomfyViewDialogEx, dialog_ex_get_view(app->dialog_ex));

    // Load persisted state (or init empty)
    somfy_state_load(&app->state);
    app->selected_blind = 0;
    app->selected_command = SomfyCmdStop;

    return app;
}

static void somfy_app_free(SomfyApp* app) {
    furi_assert(app);

    // Remove views
    view_dispatcher_remove_view(app->view_dispatcher, SomfyViewSubmenu);
    view_dispatcher_remove_view(app->view_dispatcher, SomfyViewTextInput);
    view_dispatcher_remove_view(app->view_dispatcher, SomfyViewPopup);
    view_dispatcher_remove_view(app->view_dispatcher, SomfyViewDialogEx);

    // Free views
    submenu_free(app->submenu);
    text_input_free(app->text_input);
    popup_free(app->popup);
    dialog_ex_free(app->dialog_ex);

    // Free managers
    scene_manager_free(app->scene_manager);
    view_dispatcher_free(app->view_dispatcher);

    // Close records
    furi_record_close(RECORD_GUI);
    furi_record_close(RECORD_NOTIFICATION);

    free(app);
}

int32_t somfy_rts_app(void* p) {
    UNUSED(p);

    SomfyApp* app = somfy_app_alloc();

    scene_manager_next_scene(app->scene_manager, SomfySceneMainMenu);
    view_dispatcher_run(app->view_dispatcher);

    somfy_app_free(app);
    return 0;
}
