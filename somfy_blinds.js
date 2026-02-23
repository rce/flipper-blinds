// Somfy RTS Blind Controller for Flipper Zero
print("Somfy Blinds starting, nyaa~");
let storage = require("storage");
let subghz = require("subghz");
let eventLoop = require("event_loop");
let gui = require("gui");
let submenuView = require("gui/submenu");
let textInputView = require("gui/text_input");

// Command constants
let CMD_STOP = 0x1;
let CMD_UP   = 0x2;
let CMD_DOWN = 0x4;
let CMD_PROG = 0x8;

// Build plaintext 7-byte Somfy RTS frame
function buildFrame(command, rollingCode, address) {
    let frame = [0, 0, 0, 0, 0, 0, 0];

    // Byte 0: key (upper nibble 0xA, lower nibble filled with checksum later)
    frame[0] = 0xA0;

    // Byte 1: command in upper nibble
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

// Obfuscation: XOR each byte with previous obfuscated byte
function obfuscate(frame) {
    let result = [frame[0]];
    for (let i = 1; i < 7; i++) {
        result.push(frame[i] ^ result[i - 1]);
    }
    return result;
}

// Manchester encoding constants
let SYMBOL_US = 1208;
let HALF_SYMBOL_US = 604;

// Manchester encode: rising edge (low->high) = 1, falling edge (high->low) = 0
function manchesterEncode(frame) {
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

// Transmission timing constants
let WAKEUP_HIGH = 9415;
let WAKEUP_LOW = 89565;
let HW_SYNC_HIGH = 2416;
let HW_SYNC_LOW = 2416;
let SW_SYNC_HIGH = 4550;
let SW_SYNC_LOW = 604;
let INTER_FRAME_GAP = 30415;

// Build full transmission with sync pulses and repeats
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

let SUB_FILE_PATH = "/ext/apps_data/somfy_blinds/temp.sub";
let DATA_DIR = "/ext/apps_data/somfy_blinds";
let STATE_FILE = DATA_DIR + "/state.json";

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

function transmitCommand(command, rollingCode, address) {
    let timings = buildTransmission(command, rollingCode, address, 4);
    writeSubFile(timings);

    subghz.setup();
    let result = subghz.transmitFile(SUB_FILE_PATH);
    return result;
}

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

function addBlind(name) {
    let state = loadState();
    // Generate unique 24-bit address offset by blind count
    let addr = 0x100001 + state.blinds.length + 1;
    let blind = {
        name: name,
        address: addr,
        rollingCode: 1
    };
    state.blinds.push(blind);
    saveState(state);
    return blind;
}

function sendCommand(blindIndex, command) {
    let state = loadState();
    let blind = state.blinds[blindIndex];

    let result = transmitCommand(command, blind.rollingCode, blind.address);

    // Increment and save rolling code regardless of result
    blind.rollingCode = (blind.rollingCode + 1) & 0xFFFF;
    saveState(state);

    return result;
}

// --- GUI ---

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

    eventLoop.subscribe(mainMenu.chosen, function(_sub, index) {
        let st = loadState();
        if (index < st.blinds.length) {
            showControlMenu(index);
        } else {
            showAddBlind();
        }
    });

    gui.viewDispatcher.switchTo(mainMenu);
}

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

// App entry
ensureDataDir();
showMainMenu();
eventLoop.run();
