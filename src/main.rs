#![no_main]
#![no_std]

extern crate flipperzero_alloc;
extern crate flipperzero_rt;

mod protocol;
mod storage;
mod subghz;

use core::ffi::CStr;
use flipperzero::dialogs::{DialogMessage, DialogMessageButton, DialogsApp};
use flipperzero::gui::canvas::Align;
use flipperzero::notification::{NotificationApp, led};
use flipperzero_rt::{entry, manifest};

use protocol::SomfyCommand;
use storage::{SomfyBlind, SomfyState, MAX_BLINDS, MAX_NAME_LEN};

manifest!(
    name = "Somfy Blinds Rust",
    app_version = 1,
    has_icon = false,
);

entry!(main);

fn main(_args: Option<&CStr>) -> i32 {
    let mut notif = NotificationApp::open();
    let mut dialogs = DialogsApp::open();

    flipperzero::info!("Somfy Blinds Rust starting up, meow~");

    // Load persisted state
    let mut state = storage::load_state();
    let mut selected: usize = 0;

    notif.notify(&led::ONLY_GREEN);

    loop {
        if state.blinds.is_empty() {
            // No blinds — offer to add one
            match show_empty_menu(&mut dialogs) {
                Action::AddBlind => {
                    add_blind(&mut state);
                    let _ = storage::save_state(&state);
                }
                Action::Exit => break,
                _ => {}
            }
        } else {
            // Show blind selection
            if selected >= state.blinds.len() {
                selected = 0;
            }
            match show_blind_select(&mut dialogs, &state, selected) {
                Action::PrevBlind => {
                    if selected == 0 {
                        selected = state.blinds.len() - 1;
                    } else {
                        selected -= 1;
                    }
                }
                Action::NextBlind => {
                    selected = (selected + 1) % state.blinds.len();
                }
                Action::SelectBlind => {
                    // Enter control mode for this blind
                    control_loop(&mut dialogs, &mut notif, &mut state, selected);
                    let _ = storage::save_state(&state);
                }
                Action::Exit => break,
                _ => {}
            }
        }
    }

    flipperzero::info!("Bye bye, nyaa~ :3");
    notif.notify_blocking(&led::RESET_RGB);
    0
}

/// Actions that dialogs can produce.
enum Action {
    Exit,
    PrevBlind,
    NextBlind,
    SelectBlind,
    AddBlind,
    Up,
    Down,
    Stop,
    Pair,
    Remove,
    #[allow(dead_code)]
    More,
    Back,
}

/// Show the "no blinds" menu.
fn show_empty_menu(dialogs: &mut DialogsApp) -> Action {
    let mut msg = DialogMessage::new();
    msg.set_header(c"Somfy Blinds", 0, 0, Align::Left, Align::Top);
    msg.set_text(c"No blinds yet!\nPress OK to add one", 0, 26, Align::Left, Align::Top);
    msg.set_buttons(None, Some(c"Add"), None);

    match dialogs.show_message(&msg) {
        DialogMessageButton::Center => Action::AddBlind,
        DialogMessageButton::Back => Action::Exit,
        _ => Action::Exit,
    }
}

/// Show blind selection dialog: Prev / [name] / Next.
fn show_blind_select(dialogs: &mut DialogsApp, state: &SomfyState, selected: usize) -> Action {
    let blind = &state.blinds[selected];

    // Build display text with blind name and index
    let mut text_buf = [0u8; 40];
    let _text_len = fmt_blind_info(&mut text_buf, blind, selected, state.blinds.len());
    let text_cstr = unsafe { CStr::from_ptr(text_buf.as_ptr() as *const _) };

    let mut msg = DialogMessage::new();
    msg.set_header(c"Somfy Blinds", 0, 0, Align::Left, Align::Top);
    msg.set_text(text_cstr, 0, 26, Align::Left, Align::Top);

    if state.blinds.len() > 1 {
        msg.set_buttons(Some(c"<"), Some(c"OK"), Some(c">"));
    } else {
        msg.set_buttons(None, Some(c"OK"), Some(c"+"));
    }

    match dialogs.show_message(&msg) {
        DialogMessageButton::Left => {
            if state.blinds.len() > 1 {
                Action::PrevBlind
            } else {
                Action::Exit
            }
        }
        DialogMessageButton::Center => Action::SelectBlind,
        DialogMessageButton::Right => {
            if state.blinds.len() > 1 {
                Action::NextBlind
            } else {
                // "+" button — add a blind
                Action::AddBlind
            }
        }
        DialogMessageButton::Back => Action::Exit,
    }
}

/// Control loop for a selected blind.
fn control_loop(
    dialogs: &mut DialogsApp,
    notif: &mut NotificationApp,
    state: &mut SomfyState,
    selected: usize,
) {
    loop {
        let blind = &state.blinds[selected];
        let mut name_buf = [0u8; MAX_NAME_LEN + 1];
        let name_bytes = blind.name.as_bytes();
        let len = name_bytes.len().min(MAX_NAME_LEN);
        name_buf[..len].copy_from_slice(&name_bytes[..len]);
        let name_cstr = unsafe { CStr::from_ptr(name_buf.as_ptr() as *const _) };

        let mut msg = DialogMessage::new();
        msg.set_header(name_cstr, 0, 0, Align::Left, Align::Top);
        msg.set_text(c"Control blind", 0, 26, Align::Left, Align::Top);
        msg.set_buttons(Some(c"Up"), Some(c"Stop"), Some(c"Down"));

        let action = match dialogs.show_message(&msg) {
            DialogMessageButton::Left => Action::Up,
            DialogMessageButton::Center => Action::Stop,
            DialogMessageButton::Right => Action::Down,
            DialogMessageButton::Back => Action::Back,
        };

        match action {
            Action::Up => do_transmit(notif, state, selected, SomfyCommand::Up),
            Action::Stop => do_transmit(notif, state, selected, SomfyCommand::Stop),
            Action::Down => do_transmit(notif, state, selected, SomfyCommand::Down),
            Action::Back => {
                // Show more options or go back
                match show_more_options(dialogs) {
                    Action::Pair => do_transmit(notif, state, selected, SomfyCommand::Prog),
                    Action::Remove => {
                        remove_blind(state, selected);
                        return;
                    }
                    Action::AddBlind => {
                        add_blind(state);
                        return;
                    }
                    _ => return,
                }
            }
            _ => {}
        }
    }
}

/// Show additional options: Pair / Remove / Back.
fn show_more_options(dialogs: &mut DialogsApp) -> Action {
    let mut msg = DialogMessage::new();
    msg.set_header(c"More Options", 0, 0, Align::Left, Align::Top);
    msg.set_text(c"Pair, add or remove?", 0, 26, Align::Left, Align::Top);
    msg.set_buttons(Some(c"Pair"), Some(c"+Add"), Some(c"Rm"));

    match dialogs.show_message(&msg) {
        DialogMessageButton::Left => Action::Pair,
        DialogMessageButton::Center => Action::AddBlind,
        DialogMessageButton::Right => Action::Remove,
        DialogMessageButton::Back => Action::Back,
    }
}

/// Transmit a command and update rolling code.
fn do_transmit(
    notif: &mut NotificationApp,
    state: &mut SomfyState,
    selected: usize,
    command: SomfyCommand,
) {
    let blind = &state.blinds[selected];
    flipperzero::info!("TX: addr={} rc={}", blind.address, blind.rolling_code);

    let success = subghz::transmit(command, blind.rolling_code, blind.address, 4);

    if success {
        notif.notify(&led::ONLY_GREEN);
        // Increment rolling code
        let blind = &mut state.blinds[selected];
        blind.rolling_code = blind.rolling_code.wrapping_add(1);
        if blind.rolling_code == 0 {
            blind.rolling_code = 1;
        }
        let _ = storage::save_state(state);
        flipperzero::info!("TX success, new rc={}", state.blinds[selected].rolling_code);
    } else {
        notif.notify(&led::ONLY_RED);
        flipperzero::error!("TX failed!");
    }
}

/// Add a new blind with auto-generated name and address.
fn add_blind(state: &mut SomfyState) {
    if state.blinds.len() >= MAX_BLINDS {
        return;
    }

    let index = state.blinds.len();
    let mut name = heapless::String::<MAX_NAME_LEN>::new();
    let _ = name.push_str("Blind ");
    // Simple number formatting without alloc
    let num = index + 1;
    if num >= 10 {
        let _ = name.push(char::from(b'0' + (num / 10) as u8));
    }
    let _ = name.push(char::from(b'0' + (num % 10) as u8));

    let blind = SomfyBlind {
        name,
        address: 0x100001 + (index as u32) + 1,
        rolling_code: 1,
    };
    let _ = state.blinds.push(blind);
    flipperzero::info!("Added blind {} at address {}", index + 1, 0x100001 + (index as u32) + 1);
}

/// Remove a blind by index, shifting others down.
fn remove_blind(state: &mut SomfyState, index: usize) {
    if index < state.blinds.len() {
        state.blinds.remove(index);
        flipperzero::info!("Removed blind {}", index);
    }
}

/// Format blind info into a buffer: "Name\n(1/3) addr:0x100002"
fn fmt_blind_info(buf: &mut [u8; 40], blind: &SomfyBlind, _index: usize, _total: usize) -> usize {
    let mut pos = 0;

    // Copy name
    for &b in blind.name.as_bytes() {
        if pos >= 38 {
            break;
        }
        buf[pos] = b;
        pos += 1;
    }

    // Null terminator
    buf[pos] = 0;
    pos
}
