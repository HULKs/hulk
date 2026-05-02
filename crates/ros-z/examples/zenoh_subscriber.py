#!/usr/bin/env python3
"""
External Zenoh subscriber demonstrating CDR metadata from ros-z.

This Python script subscribes to a ros-z topic and prints the CDR encoding
metadata exposed over Zenoh.

Requirements:
    pip install zenoh

Usage:
    # Terminal 1: Start a ros-z publisher
    cargo run --example z_pubsub -- pub

    # Terminal 2: Run this script
    python3 examples/zenoh_subscriber.py
"""

import zenoh
import time
import sys


def main():
    print("=== External Zenoh Subscriber ===\n")
    print("Demonstrating ros-z CDR encoding metadata\n")

    # Open Zenoh session
    conf = zenoh.Config()
    session = zenoh.open(conf)

    print("Zenoh session opened")
    print("\nSubscribing to:")
    print("  - /interop/cdr (expects CDR encoding)")
    print("\nWaiting for messages...\n")

    def cdr_callback(sample):
        """Handle CDR-encoded messages"""
        encoding = sample.encoding
        payload = sample.payload.to_bytes()

        print("[CDR]   Received message")
        print(f"        Encoding: {encoding}")
        print(f"        Payload size: {len(payload)} bytes")
        print(f"        Raw data: {payload[:50]}...")  # Show first 50 bytes
        print()

    sub_cdr = session.declare_subscriber("/interop/cdr", cdr_callback)

    print("✓ Subscriptions active\n")
    print("Press Ctrl+C to exit\n")

    try:
        # Keep alive
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        print("\n\nShutting down...")

    # Cleanup
    sub_cdr.undeclare()
    session.close()

    print("=== Complete ===")


if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
