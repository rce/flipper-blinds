#pragma once

#include "../somfy_rts.h"

// Main Menu scene
void somfy_scene_main_menu_on_enter(void* context);
bool somfy_scene_main_menu_on_event(void* context, SceneManagerEvent event);
void somfy_scene_main_menu_on_exit(void* context);

// Control scene
void somfy_scene_control_on_enter(void* context);
bool somfy_scene_control_on_event(void* context, SceneManagerEvent event);
void somfy_scene_control_on_exit(void* context);

// Add Blind scene
void somfy_scene_add_blind_on_enter(void* context);
bool somfy_scene_add_blind_on_event(void* context, SceneManagerEvent event);
void somfy_scene_add_blind_on_exit(void* context);

// Transmit scene
void somfy_scene_transmit_on_enter(void* context);
bool somfy_scene_transmit_on_event(void* context, SceneManagerEvent event);
void somfy_scene_transmit_on_exit(void* context);

// Confirm Remove scene
void somfy_scene_confirm_remove_on_enter(void* context);
bool somfy_scene_confirm_remove_on_event(void* context, SceneManagerEvent event);
void somfy_scene_confirm_remove_on_exit(void* context);
