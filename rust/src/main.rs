#![no_main]
#![no_std]

extern crate flipperzero_rt;

use core::ffi::CStr;
use flipperzero::dialogs::{DialogMessage, DialogMessageButton, DialogsApp};
use flipperzero::gui::canvas::Align;
use flipperzero::notification::{NotificationApp, led};
use flipperzero_rt::{entry, manifest};

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

    // Blink green on startup
    notif.notify(&led::ONLY_GREEN);

    loop {
        let mut msg = DialogMessage::new();
        msg.set_header(c"Hello from Rust! :3", 0, 0, Align::Left, Align::Top);
        msg.set_text(c"Meow~ Press a button!", 0, 26, Align::Left, Align::Top);
        msg.set_buttons(Some(c"Nya"), Some(c"Purr"), Some(c"Meow"));

        let button = dialogs.show_message(&msg);

        match button {
            DialogMessageButton::Left => {
                flipperzero::info!("Nya! (green)");
                notif.notify(&led::ONLY_GREEN);
            }
            DialogMessageButton::Center => {
                flipperzero::info!("Purr~ (blue)");
                notif.notify(&led::ONLY_BLUE);
            }
            DialogMessageButton::Right => {
                flipperzero::info!("Meow! (red)");
                notif.notify(&led::ONLY_RED);
            }
            DialogMessageButton::Back => break,
        }
    }

    flipperzero::info!("Bye bye, nyaa~ :3");
    // Clean exit
    notif.notify_blocking(&led::RESET_RGB);
    0
}
