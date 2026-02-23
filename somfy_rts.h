#pragma once

#include <furi.h>
#include <gui/gui.h>
#include <gui/scene_manager.h>
#include <gui/view_dispatcher.h>
#include <gui/modules/submenu.h>
#include <gui/modules/text_input.h>
#include <gui/modules/popup.h>
#include <notification/notification_messages.h>

#define SOMFY_MAX_BLINDS 8
#define SOMFY_MAX_NAME_LEN 21 // 20 chars + null
#define SOMFY_TX_REPEATS 4

// Scenes
typedef enum {
    SomfySceneMainMenu,
    SomfySceneControl,
    SomfySceneAddBlind,
    SomfySceneTransmit,
    SomfySceneCount,
} SomfyScene;

// Views
typedef enum {
    SomfyViewSubmenu,
    SomfyViewTextInput,
    SomfyViewPopup,
} SomfyView;

// Somfy RTS commands
typedef enum {
    SomfyCmdStop = 0x1,
    SomfyCmdUp = 0x2,
    SomfyCmdDown = 0x4,
    SomfyCmdProg = 0x8,
} SomfyCommand;

// Custom events for scene manager
typedef enum {
    SomfyEventBlindSelected,
    SomfyEventAddBlind,
    SomfyEventCommandSelected,
    SomfyEventNameEntered,
    SomfyEventTxDone,
} SomfyEvent;

// Blind data
typedef struct {
    char name[SOMFY_MAX_NAME_LEN];
    uint32_t address;
    uint16_t rolling_code;
} SomfyBlind;

// Persisted state
typedef struct {
    SomfyBlind blinds[SOMFY_MAX_BLINDS];
    uint8_t count;
} SomfyState;

// Main app struct
typedef struct {
    SceneManager* scene_manager;
    ViewDispatcher* view_dispatcher;
    Submenu* submenu;
    TextInput* text_input;
    Popup* popup;
    NotificationApp* notifications;
    Gui* gui;

    SomfyState state;
    uint8_t selected_blind;
    SomfyCommand selected_command;
    char text_input_buf[SOMFY_MAX_NAME_LEN];
} SomfyApp;
