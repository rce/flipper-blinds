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
