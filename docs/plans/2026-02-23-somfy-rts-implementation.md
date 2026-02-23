# Somfy RTS Flipper Zero Controller — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Flipper Zero JS app that pairs with and controls Somfy RTS rolling blinds.

**Architecture:** Single JS file generates Somfy RTS protocol frames, writes them as RAW `.sub` files to the SD card, and transmits via the `subghz` module. State (rolling codes, blind configs) persisted as JSON. Event-driven GUI using Flipper's `gui/submenu` and `gui/text_input` views.

**Tech Stack:** Flipper Zero JavaScript runtime (stock firmware), Sub-GHz radio (CC1101), SD card storage.

**Deployment:** Copy `somfy_blinds.js` to Flipper via `python3 deploy.py` script that uses serial CLI, or manually via qFlipper.

---

### Task 1: Project Setup and Deploy Script

**Files:**
- Create: `somfy_blinds.js` (skeleton)
- Create: `deploy.py` (USB deploy helper)

**Step 1: Create minimal JS skeleton**

Create `somfy_blinds.js` with just a print statement to verify deployment works:

```javascript
// Somfy RTS Blind Controller for Flipper Zero
print("Somfy Blinds starting, nyaa~");
```

**Step 2: Create deploy script**

Create `deploy.py` — a Python script that copies the JS file to the Flipper over USB serial. Uses pyserial to send CLI `storage write` commands:

```python
#!/usr/bin/env python3
"""Deploy somfy_blinds.js to Flipper Zero over USB serial."""
import serial
import sys
import os
import time

DEFAULT_PORT_PATTERNS = ["/dev/tty.usbmodem*", "/dev/ttyACM*"]
DEST_PATH = "/ext/apps/Scripts/somfy_blinds.js"
SRC_FILE = "somfy_blinds.js"

def find_port():
    import glob
    for pattern in DEFAULT_PORT_PATTERNS:
        matches = glob.glob(pattern)
        if matches:
            return matches[0]
    return None

def deploy(port_path, src_file):
    with open(src_file, "r") as f:
        content = f.read()

    ser = serial.Serial(port_path, baudrate=230400, timeout=2)
    time.sleep(0.5)

    # Send Ctrl+C to break any running command
    ser.write(b"\x03")
    time.sleep(0.3)
    ser.read(ser.in_waiting)  # flush

    # Write file using storage write command
    cmd = f'storage write_chunk {DEST_PATH} {len(content.encode("utf-8"))}\r'
    ser.write(cmd.encode())
    time.sleep(0.3)
    ser.write(content.encode("utf-8"))
    time.sleep(0.5)

    response = ser.read(ser.in_waiting).decode(errors="replace")
    ser.close()
    print(f"Deployed {src_file} -> {DEST_PATH}")
    print(f"Response: {response}")

if __name__ == "__main__":
    port = sys.argv[1] if len(sys.argv) > 1 else find_port()
    if not port:
        print("No Flipper found. Pass port as argument.")
        sys.exit(1)
    src = sys.argv[2] if len(sys.argv) > 2 else SRC_FILE
    deploy(port, src)
```

**Step 3: Test deployment**

Run: `pip install pyserial && python3 deploy.py`

Connect to Flipper, navigate to Scripts, run `somfy_blinds.js`. Verify it prints
the startup message.

**Step 4: Commit**

```bash
git add somfy_blinds.js deploy.py
git commit -m "feat: bootstrap project with deploy script — the cat's out of the bag"
```

---

### Task 2: Somfy RTS Frame Encoding

This is the core protocol logic. We build the 7-byte Somfy RTS frame, apply
obfuscation, and Manchester-encode it into RF timing values.

**Files:**
- Modify: `somfy_blinds.js`

**Step 1: Implement frame builder**

Add function to build the plaintext 7-byte Somfy RTS frame:

```javascript
// Command constants
let CMD_STOP = 0x1;
let CMD_UP   = 0x2;
let CMD_DOWN = 0x4;
let CMD_PROG = 0x8;

function buildFrame(command, rollingCode, address) {
    let frame = [0, 0, 0, 0, 0, 0, 0];

    // Byte 0: key (upper nibble 0xA, lower nibble filled with checksum later)
    frame[0] = 0xA0;

    // Byte 1: command in upper nibble, lower nibble is part of rolling code? No —
    // Byte 1 upper nibble = command, lower nibble = checksum target
    // Actually per protocol: byte 1 = command << 4
    frame[1] = (command & 0x0F) << 4;

    // Bytes 2-3: rolling code (big-endian)
    frame[2] = (rollingCode >> 8) & 0xFF;
    frame[3] = rollingCode & 0xFF;

    // Bytes 4-6: address (big-endian)
    frame[4] = (address >> 16) & 0xFF;
    frame[5] = (address >> 8) & 0xFF;
    frame[6] = address & 0xFF;

    // Checksum: XOR all nibbles
    let checksum = 0;
    for (let i = 0; i < 7; i++) {
        checksum = checksum ^ (frame[i] >> 4) ^ (frame[i] & 0x0F);
    }
    frame[1] = frame[1] | (checksum & 0x0F);

    return frame;
}
```

**Step 2: Implement obfuscation**

```javascript
function obfuscate(frame) {
    let result = [frame[0]];
    for (let i = 1; i < 7; i++) {
        result.push(frame[i] ^ frame[i - 1]);
    }
    return result;
}
```

**Step 3: Implement Manchester encoding to timing values**

Manchester encoding: rising edge (low→high) = 1, falling edge (high→low) = 0.
Symbol width ~1208 us, so half-symbol ~604 us.

```javascript
let SYMBOL_US = 1208;
let HALF_SYMBOL_US = 604;

function manchesterEncode(frame) {
    // Returns array of [high_us, low_us] timing pairs
    let timings = [];
    for (let i = 0; i < 7; i++) {
        for (let bit = 7; bit >= 0; bit--) {
            let b = (frame[i] >> bit) & 1;
            if (b === 1) {
                // Rising edge: low then high
                timings.push(-HALF_SYMBOL_US);
                timings.push(HALF_SYMBOL_US);
            } else {
                // Falling edge: high then low
                timings.push(HALF_SYMBOL_US);
                timings.push(-HALF_SYMBOL_US);
            }
        }
    }
    return timings;
}
```

**Step 4: Build complete transmission with sync pulses**

```javascript
let WAKEUP_HIGH = 9415;
let WAKEUP_LOW = 89565;
let HW_SYNC_HIGH = 2416;
let HW_SYNC_LOW = 2416;
let SW_SYNC_HIGH = 4550;
let SW_SYNC_LOW = 604;
let INTER_FRAME_GAP = 30415;

function buildTransmission(command, rollingCode, address, repeats) {
    let frame = buildFrame(command, rollingCode, address);
    let obfuscated = obfuscate(frame);
    let manchester = manchesterEncode(obfuscated);

    let timings = [];

    for (let r = 0; r < repeats; r++) {
        if (r === 0) {
            // First frame: wakeup + 2x hardware sync
            timings.push(WAKEUP_HIGH);
            timings.push(-WAKEUP_LOW);
            for (let s = 0; s < 2; s++) {
                timings.push(HW_SYNC_HIGH);
                timings.push(-HW_SYNC_LOW);
            }
        } else {
            // Repeat frames: 7x hardware sync
            for (let s = 0; s < 7; s++) {
                timings.push(HW_SYNC_HIGH);
                timings.push(-HW_SYNC_LOW);
            }
        }

        // Software sync
        timings.push(SW_SYNC_HIGH);
        timings.push(-SW_SYNC_LOW);

        // Manchester-encoded data
        // Merge first timing with sw sync low if same sign
        for (let t = 0; t < manchester.length; t++) {
            timings.push(manchester[t]);
        }

        // Inter-frame gap (except after last repeat)
        if (r < repeats - 1) {
            timings.push(-INTER_FRAME_GAP);
        }
    }

    return timings;
}
```

**Step 5: Add a quick self-test with print output**

```javascript
// Self-test: print a frame for known values
let testFrame = buildFrame(CMD_UP, 1, 0x123456);
let testOb = obfuscate(testFrame);
print("Frame:", testFrame.join(","));
print("Obfuscated:", testOb.join(","));
```

**Step 6: Deploy and verify**

Run: `python3 deploy.py`
Run the script on Flipper — check that frame bytes print correctly.

**Step 7: Commit**

```bash
git add somfy_blinds.js
git commit -m "feat: implement Somfy RTS frame encoding — purrfect protocol"
```

---

### Task 3: .sub File Generation and Transmission

**Files:**
- Modify: `somfy_blinds.js`

**Step 1: Implement .sub file writer**

```javascript
let storage = require("storage");
let subghz = require("subghz");

let SUB_FILE_PATH = "/ext/apps_data/somfy_blinds/temp.sub";
let DATA_DIR = "/ext/apps_data/somfy_blinds";

function ensureDataDir() {
    if (!storage.directoryExists(DATA_DIR)) {
        storage.makeDirectory(DATA_DIR);
    }
}

function writeSubFile(timings) {
    ensureDataDir();

    // Build .sub file content
    let header = "Filetype: Flipper SubGhz RAW File\n";
    header += "Version: 1\n";
    header += "Frequency: 433420000\n";
    header += "Preset: FuriHalSubGhzPresetOok650Async\n";
    header += "Protocol: RAW\n";

    // Split timings into lines of max 512 values
    let lines = "";
    let lineValues = [];
    for (let i = 0; i < timings.length; i++) {
        lineValues.push(timings[i].toString());
        if (lineValues.length >= 512) {
            lines += "RAW_Data: " + lineValues.join(" ") + "\n";
            lineValues = [];
        }
    }
    if (lineValues.length > 0) {
        lines += "RAW_Data: " + lineValues.join(" ") + "\n";
    }

    let content = header + lines;

    // Write to file
    if (storage.fileExists(SUB_FILE_PATH)) {
        storage.remove(SUB_FILE_PATH);
    }
    let file = storage.openFile(SUB_FILE_PATH, "w", "create_always");
    file.write(content);
    file.close();
}
```

**Step 2: Implement transmit function**

```javascript
function transmitCommand(command, rollingCode, address) {
    let timings = buildTransmission(command, rollingCode, address, 4);
    writeSubFile(timings);

    subghz.setup();
    let result = subghz.transmitFile(SUB_FILE_PATH);
    return result;
}
```

**Step 3: Test with a dummy transmission**

Add temporary test code:

```javascript
ensureDataDir();
print("Generating test signal...");
let timings = buildTransmission(CMD_UP, 1, 0x654321, 4);
writeSubFile(timings);
print("Sub file written, " + timings.length.toString() + " timing values");
subghz.setup();
let ok = subghz.transmitFile(SUB_FILE_PATH);
print("Transmit result: " + ok.toString());
```

**Step 4: Deploy and test**

Run: `python3 deploy.py`
Run on Flipper — verify .sub file is written and transmit returns true.
(No blind will respond yet since we haven't paired, but the radio should transmit.)

**Step 5: Commit**

```bash
git add somfy_blinds.js
git commit -m "feat: generate .sub files and transmit — radio cats unite"
```

---

### Task 4: State Persistence (Rolling Codes)

**Files:**
- Modify: `somfy_blinds.js`

**Step 1: Implement state load/save**

```javascript
let STATE_FILE = DATA_DIR + "/state.json";

function loadState() {
    ensureDataDir();
    if (!storage.fileExists(STATE_FILE)) {
        return { blinds: [] };
    }
    let file = storage.openFile(STATE_FILE, "r", "open_existing");
    let content = file.read("ascii", file.size());
    file.close();
    return JSON.parse(content);
}

function saveState(state) {
    ensureDataDir();
    if (storage.fileExists(STATE_FILE)) {
        storage.remove(STATE_FILE);
    }
    let file = storage.openFile(STATE_FILE, "w", "create_always");
    file.write(JSON.stringify(state));
    file.close();
}
```

**Step 2: Implement add blind**

```javascript
function generateAddress() {
    // Generate a pseudo-random 24-bit address
    // Use a simple approach since Math.random may not be available
    let addr = 0x100001;
    let state = loadState();
    // Offset by number of existing blinds to ensure unique
    addr = addr + state.blinds.length + 1;
    return addr;
}

function addBlind(name) {
    let state = loadState();
    let blind = {
        name: name,
        address: generateAddress(),
        rollingCode: 1
    };
    state.blinds.push(blind);
    saveState(state);
    return blind;
}
```

**Step 3: Implement send command with rolling code increment**

```javascript
function sendCommand(blindIndex, command) {
    let state = loadState();
    let blind = state.blinds[blindIndex];

    let result = transmitCommand(command, blind.rollingCode, blind.address);

    // Increment and save rolling code regardless of result
    blind.rollingCode = (blind.rollingCode + 1) & 0xFFFF;
    saveState(state);

    return result;
}
```

**Step 4: Deploy, test adding a blind and sending a command**

Temporarily add test code, deploy, run on Flipper. Check that `state.json`
is created with correct structure.

**Step 5: Commit**

```bash
git add somfy_blinds.js
git commit -m "feat: persist blind state and rolling codes — never fur-get"
```

---

### Task 5: GUI — Main Menu and Blind Control

**Files:**
- Modify: `somfy_blinds.js`

**Step 1: Implement main menu**

Replace test code with the event-driven GUI:

```javascript
let eventLoop = require("event_loop");
let gui = require("gui");
let submenuView = require("gui/submenu");

// Build main menu items from state
function showMainMenu() {
    let state = loadState();
    let items = [];
    for (let i = 0; i < state.blinds.length; i++) {
        items.push(state.blinds[i].name);
    }
    items.push("+ Add Blind");

    let mainMenu = submenuView.makeWith({
        header: "Somfy Blinds"
    }, items);

    eventLoop.subscribe(mainMenu.chosen, function(_sub, index, _gui, _evLoop) {
        let st = loadState();
        if (index < st.blinds.length) {
            showControlMenu(index);
        } else {
            showAddBlind();
        }
    }, gui, eventLoop);

    gui.viewDispatcher.switchTo(mainMenu);
}
```

**Step 2: Implement control menu**

```javascript
function showControlMenu(blindIndex) {
    let state = loadState();
    let blind = state.blinds[blindIndex];

    let controlMenu = submenuView.makeWith({
        header: blind.name
    }, ["Up", "Stop", "Down", "Pair (Prog)"]);

    eventLoop.subscribe(controlMenu.chosen, function(_sub, index) {
        let commands = [CMD_UP, CMD_STOP, CMD_DOWN, CMD_PROG];
        let cmdNames = ["Up", "Stop", "Down", "Pair"];
        print("Sending " + cmdNames[index] + "...");
        sendCommand(blindIndex, commands[index]);
        print("Sent!");
    });

    eventLoop.subscribe(gui.viewDispatcher.navigation, function() {
        showMainMenu();
    });

    gui.viewDispatcher.switchTo(controlMenu);
}
```

**Step 3: Implement add blind screen**

```javascript
let textInputView = require("gui/text_input");

function showAddBlind() {
    let nameInput = textInputView.makeWith({
        header: "Blind Name",
        minLength: 1,
        maxLength: 20,
        defaultText: "Blind",
        defaultTextClear: true
    });

    eventLoop.subscribe(nameInput.input, function(_sub, name) {
        addBlind(name);
        print("Added blind: " + name);
        showMainMenu();
    });

    eventLoop.subscribe(gui.viewDispatcher.navigation, function() {
        showMainMenu();
    });

    gui.viewDispatcher.switchTo(nameInput);
}
```

**Step 4: Wire up app entry point**

Replace the skeleton entry code with:

```javascript
// App entry
ensureDataDir();
showMainMenu();
eventLoop.run();
```

**Step 5: Deploy and test**

Run: `python3 deploy.py`
On Flipper: run the script, verify menu appears with "Add Blind" option,
add a blind, verify control menu shows Up/Stop/Down/Pair.

**Step 6: Commit**

```bash
git add somfy_blinds.js
git commit -m "feat: add GUI menus for blind selection and control — feline fine"
```

---

### Task 6: Integration Testing and Polish

**Files:**
- Modify: `somfy_blinds.js`

**Step 1: Test the full pairing flow**

1. Run app on Flipper
2. Add a new blind ("Test Blind")
3. Put Somfy motor into programming mode (hold prog button on motor head)
4. Select the blind → "Pair (Prog)"
5. Motor should jog to confirm pairing

**Step 2: Test control commands**

After successful pairing:
1. Select the blind → "Up" → blind should go up
2. Select the blind → "Stop" → blind should stop
3. Select the blind → "Down" → blind should go down

**Step 3: Fix any timing/encoding issues**

If the motor doesn't respond, likely issues are:
- Manchester encoding polarity (try swapping 0/1 encoding)
- Timing values (try ±10% adjustments on SYMBOL_US)
- Checksum calculation (verify against reference implementations)
- Missing or incorrect inter-frame timing consolidation

Reference implementations for comparison:
- https://github.com/Nickduino/Somfy_Remote (Arduino)
- https://github.com/loopj/open-rts (C)

**Step 4: Add notification feedback**

```javascript
let notify = require("notification");

// In sendCommand, after transmit:
notify.blink("green", "short");
// Or on failure:
notify.blink("red", "short");
```

**Step 5: Commit**

```bash
git add somfy_blinds.js
git commit -m "feat: polish and integrate — the purrfect blind controller"
```

---

### Task 7: Deploy Script Refinement

**Files:**
- Modify: `deploy.py`

**Step 1: Add run-after-deploy option**

Enhance `deploy.py` to optionally run the script on the Flipper after deploying:

```python
# Add --run flag to auto-launch the script after deploy
# Send: js /ext/apps/Scripts/somfy_blinds.js
```

**Step 2: Add serial port auto-detection feedback**

Print which port was found, or list available ports if none match.

**Step 3: Test full workflow**

```bash
python3 deploy.py          # deploy only
python3 deploy.py --run    # deploy and run
```

**Step 4: Commit**

```bash
git add deploy.py
git commit -m "feat: deploy script with --run flag — cat-apult to Flipper"
```
