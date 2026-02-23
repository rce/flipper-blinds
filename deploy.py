#!/usr/bin/env python3
"""Deploy somfy_blinds.js to Flipper Zero over USB serial."""
import serial
import sys
import os
import time
import glob
import argparse

DEFAULT_PORT_PATTERNS = ["/dev/tty.usbmodem*", "/dev/ttyACM*"]
DEST_PATH = "/ext/apps/Scripts/somfy_blinds.js"
SRC_FILE = "somfy_blinds.js"


def find_port():
    for pattern in DEFAULT_PORT_PATTERNS:
        matches = glob.glob(pattern)
        if matches:
            return matches[0]
    return None


def list_ports():
    """List all potentially matching serial ports."""
    all_matches = []
    for pattern in DEFAULT_PORT_PATTERNS:
        all_matches.extend(glob.glob(pattern))
    return all_matches


def send_cli_command(ser, cmd):
    """Send a CLI command and read the response."""
    ser.write((cmd + "\r").encode())
    time.sleep(0.3)
    return ser.read(ser.in_waiting).decode(errors="replace")


def deploy(port_path, src_file, run_after=False):
    with open(src_file, "r") as f:
        content = f.read()

    content_bytes = content.encode("utf-8")
    print(f"Connecting to {port_path}...")

    ser = serial.Serial(port_path, baudrate=230400, timeout=2)
    time.sleep(0.5)

    # Send Ctrl+C to break any running command
    ser.write(b"\x03")
    time.sleep(0.3)
    ser.read(ser.in_waiting)  # flush

    # Write file using storage write_chunk command
    print(f"Writing {len(content_bytes)} bytes to {DEST_PATH}...")
    cmd = f"storage write_chunk {DEST_PATH} {len(content_bytes)}\r"
    ser.write(cmd.encode())
    time.sleep(0.3)
    ser.write(content_bytes)
    time.sleep(0.5)

    response = ser.read(ser.in_waiting).decode(errors="replace")
    print(f"Deployed {src_file} -> {DEST_PATH}")

    if run_after:
        print(f"Launching script on Flipper...")
        run_cmd = f"js {DEST_PATH}"
        resp = send_cli_command(ser, run_cmd)
        print(f"Script launched. Output: {resp}")

    ser.close()


def main():
    parser = argparse.ArgumentParser(description="Deploy JS app to Flipper Zero")
    parser.add_argument("port", nargs="?", help="Serial port (auto-detected if omitted)")
    parser.add_argument("--src", default=SRC_FILE, help=f"Source JS file (default: {SRC_FILE})")
    parser.add_argument("--run", action="store_true", help="Run the script on Flipper after deploying")
    args = parser.parse_args()

    port = args.port or find_port()
    if not port:
        available = list_ports()
        if available:
            print(f"Found ports but none matched: {available}")
        else:
            print("No Flipper Zero found. Available patterns checked:")
            for p in DEFAULT_PORT_PATTERNS:
                print(f"  {p}")
            print("Pass the port as an argument: python3 deploy.py /dev/tty.xxx")
        sys.exit(1)

    print(f"Using port: {port}")
    deploy(port, args.src, run_after=args.run)


if __name__ == "__main__":
    main()
