#![no_main]
#![no_std]

extern crate flipperzero_rt;

use core::ffi::CStr;
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
    notif.notify_blocking(&led::ONLY_GREEN);

    0
}
