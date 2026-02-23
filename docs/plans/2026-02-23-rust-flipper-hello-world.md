# Rust Flipper Zero Hello World Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Get a minimal Rust app compiling and running on a Flipper Zero — display text, blink LED, exit on button press.

**Architecture:** Native `.fap` Flipper app using `flipperzero-rs` crate ecosystem. Uses the Dialog API for text display + button input (safe, ergonomic), and the Notification API for LED feedback. `no_std`, nightly Rust, `thumbv7em-none-eabihf` target.

**Tech Stack:** Rust nightly (2025-08-31), flipperzero 0.16.0, flipperzero-sys 0.16.0, flipperzero-rt 0.16.0

---

### Task 1: Set up Rust project scaffolding

**Files:**
- Create: `rust/.cargo/config.toml`
- Create: `rust/Cargo.toml`
- Create: `rust/rust-toolchain.toml`
- Create: `rust/src/main.rs` (minimal skeleton)

**Step 1: Create `.cargo/config.toml`**

```toml
[target.thumbv7em-none-eabihf]
rustflags = [
    "-C", "target-cpu=cortex-m4",
    "-C", "panic=abort",
    "-C", "debuginfo=0",
    "-C", "opt-level=z",
    "-C", "embed-bitcode=yes",
    "-C", "lto=yes",
    "-C", "link-args=--script=flipperzero-rt.ld --Bstatic --relocatable --discard-all --strip-all --lto-O3 --lto-whole-program-visibility",
]

[build]
target = "thumbv7em-none-eabihf"
```

**Step 2: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "nightly-2025-08-31"
targets = ["thumbv7em-none-eabihf"]
```

**Step 3: Create `Cargo.toml`**

```toml
cargo-features = ["different-binary-name"]

[package]
name = "somfy-blinds-rust"
version = "0.1.0"
edition = "2024"
rust-version = "1.85.0"
autobins = false
autoexamples = false
autotests = false
autobenches = false

[[bin]]
name = "somfy-blinds-rust"
filename = "somfy_blinds.fap"
bench = false
test = false

[dependencies]
flipperzero = "0.16.0"
flipperzero-sys = "0.16.0"
flipperzero-rt = "0.16.0"
```

**Step 4: Create minimal `src/main.rs`**

```rust
#![no_main]
#![no_std]

extern crate flipperzero_rt;

use core::ffi::CStr;
use flipperzero_rt::{entry, manifest};

manifest!(
    name = "Somfy Blinds Rust",
    app_version = 1,
    has_icon = false,
);

entry!(main);

fn main(_args: Option<&CStr>) -> i32 {
    0
}
```

**Step 5: Verify it compiles**

Run: `cd rust && cargo build --release`
Expected: Successful compilation producing `target/thumbv7em-none-eabihf/release/somfy_blinds.fap`

**Step 6: Commit**

```bash
git add rust/
git commit -m "feat(rust): scaffold Flipper Zero Rust project — the kitten's first steps"
```

---

### Task 2: Add LED notification on startup

**Files:**
- Modify: `rust/src/main.rs`

**Step 1: Add LED blink to main**

Replace the `main` function body in `src/main.rs`:

```rust
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
```

**Step 2: Verify it compiles**

Run: `cd rust && cargo build --release`
Expected: Successful compilation

**Step 3: Commit**

```bash
git add rust/src/main.rs
git commit -m "feat(rust): add green LED blink on startup — a glowing re-fur-ral"
```

---

### Task 3: Add dialog with text and button handling

**Files:**
- Modify: `rust/src/main.rs`

**Step 1: Replace main with dialog-based UI**

```rust
#![no_main]
#![no_std]

extern crate flipperzero_rt;

use core::ffi::CStr;
use flipperzero::dialogs::{DialogMessage, DialogMessageButton, DialogsApp};
use flipperzero::gui::canvas::Align;
use flipperzero::notification::{NotificationApp, feedback, led};
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
                notif.notify(&led::ONLY_GREEN);
            }
            DialogMessageButton::Center => {
                notif.notify(&led::ONLY_BLUE);
            }
            DialogMessageButton::Right => {
                notif.notify(&led::ONLY_RED);
            }
            DialogMessageButton::Back => break,
        }
    }

    // Clean exit
    notif.notify_blocking(&led::RESET_RGB);
    0
}
```

This shows a dialog with header text and three buttons. Each button lights a different LED color. Back button exits the app cleanly.

**Step 2: Verify it compiles**

Run: `cd rust && cargo build --release`
Expected: Successful compilation

**Step 3: Commit**

```bash
git add rust/src/main.rs
git commit -m "feat(rust): add dialog UI with button-triggered LEDs — feline interactive"
```

---

### Task 4: Test on hardware (manual)

This task is manual — requires a physical Flipper Zero connected via USB.

**Step 1: Deploy the .fap to Flipper**

Option A — using `flipperzero-tools`:
```bash
cargo install flipperzero-tools
cd rust
storage send target/thumbv7em-none-eabihf/release/somfy_blinds.fap /ext/apps/Examples/somfy_blinds.fap
```

Option B — extend the existing `deploy.py` to handle `.fap` files (future enhancement).

Option C — copy manually via qFlipper or SD card.

**Step 2: Run the app on Flipper**

Navigate to `Apps > Examples > Somfy Blinds Rust` on the Flipper and launch it.

**Step 3: Verify behavior**

- Green LED blinks on startup
- Dialog shows "Hello from Rust! :3" with three buttons
- Each button triggers a different LED color
- Back button exits cleanly

**Step 4: Commit any fixes if needed**

---

### Notes

- If the nightly date `2025-08-31` doesn't work (toolchain not available), try the latest nightly or check flipperzero-rs releases for the recommended date
- The `.fap` is pinned to a Flipper SDK version — if the Flipper firmware is too old/new, you'll see "Outdated App". Update firmware or pin crate versions accordingly
- `c"..."` string literals require Rust 1.77+ (we're on nightly, so this is fine)
- `notify()` vs `notify_blocking()`: use `notify_blocking()` only right before exit to ensure the LED sequence completes
