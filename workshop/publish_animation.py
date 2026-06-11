#!/usr/bin/env python3
"""Publish workshop arm animation frames to Zenoh."""

from __future__ import annotations

import argparse
import json
import math
import struct
import sys
import time
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any, cast

import numpy as np
from numpy.typing import NDArray

ARM_JOINTS_TOPIC = "arm_joints"
CDR_LE_HEADER = b"\x00\x01\x00\x00"
DEFAULT_ROUTER_PORT = 7447
JOINT_COUNT = 8


@dataclass(frozen=True)
class Animation:
    path: Path
    fps: float
    positions: NDArray[np.float32]


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Read a workshop animation .npy file and publish its arm joint "
            "positions to Zenoh."
        ),
    )
    parser.add_argument(
        "animation",
        type=Path,
        help="animation .npy file",
    )
    parser.add_argument(
        "--key",
        default=ARM_JOINTS_TOPIC,
        help=(
            f"Zenoh key expression to publish to (default: {ARM_JOINTS_TOPIC})"
        ),
    )
    parser.add_argument(
        "--router",
        help=(
            "Zenoh router endpoint on the robot, e.g. "
            "tcp/10.1.24.42:7447. Bare hosts are expanded to "
            f"tcp/<host>:{DEFAULT_ROUTER_PORT}."
        ),
    )
    parser.add_argument(
        "--fps",
        type=positive_float,
        help="override the animation file's fps",
    )
    parser.add_argument(
        "--repeat",
        type=non_negative_int,
        default=1,
        help="number of times to publish the animation; 0 means forever",
    )
    parser.add_argument(
        "--loop",
        action="store_true",
        help="publish the animation forever, equivalent to --repeat 0",
    )
    parser.add_argument(
        "--start-delay",
        type=non_negative_float,
        default=0.0,
        help="seconds to wait after declaring the publisher before publishing",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="validate the animation and CDR payloads without opening Zenoh",
    )
    return parser.parse_args(argv)


def positive_float(value: str) -> float:
    parsed = float(value)
    if not math.isfinite(parsed) or parsed <= 0.0:
        msg = "value must be a positive finite number"
        raise argparse.ArgumentTypeError(msg)
    return parsed


def non_negative_float(value: str) -> float:
    parsed = float(value)
    if not math.isfinite(parsed) or parsed < 0.0:
        msg = "value must be a non-negative finite number"
        raise argparse.ArgumentTypeError(msg)
    return parsed


def non_negative_int(value: str) -> int:
    parsed = int(value)
    if parsed < 0:
        msg = "value must be non-negative"
        raise argparse.ArgumentTypeError(msg)
    return parsed


def normalize_router_endpoint(value: str) -> str:
    if "/" in value:
        return value
    if ":" in value:
        return f"tcp/{value}"
    return f"tcp/{value}:{DEFAULT_ROUTER_PORT}"


def load_animation(path: Path) -> Animation:
    animation_path = path.expanduser().resolve()
    data = np.load(animation_path, allow_pickle=True).item()

    if not isinstance(data, dict):
        msg = f"{animation_path} must contain a dictionary"
        raise TypeError(msg)
    if "fps" not in data:
        msg = f"{animation_path} does not contain an 'fps' entry"
        raise ValueError(msg)
    if "positions" not in data:
        msg = f"{animation_path} does not contain a 'positions' entry"
        raise ValueError(msg)

    fps = positive_float(str(data["fps"]))
    positions64 = np.asarray(data["positions"], dtype=np.float64)

    if positions64.ndim != 2 or positions64.shape[1] != JOINT_COUNT:
        msg = (
            f"positions must have shape (frames, {JOINT_COUNT}), "
            f"got {positions64.shape}"
        )
        raise ValueError(msg)
    if positions64.shape[0] == 0:
        msg = "positions must contain at least one frame"
        raise ValueError(msg)
    if not np.isfinite(positions64).all():
        msg = "positions must contain only finite values"
        raise ValueError(msg)

    max_f32 = np.finfo(np.float32).max
    if np.abs(positions64).max() > max_f32:
        msg = "positions must fit into f32"
        raise ValueError(msg)

    return Animation(
        path=animation_path,
        fps=fps,
        positions=positions64.astype(np.float32),
    )


def serialize_arm_joints(positions: NDArray[np.float32]) -> bytes:
    if positions.shape != (JOINT_COUNT,):
        msg = f"arm joint position frame must have shape ({JOINT_COUNT},)"
        raise ValueError(msg)
    if not np.isfinite(positions).all():
        msg = "arm joint position frame must contain only finite values"
        raise ValueError(msg)

    return CDR_LE_HEADER + struct.pack("<8f", *positions.tolist())


def build_zenoh_config(zenoh: Any, router: str | None) -> Any:
    config = zenoh.Config()
    if router is None:
        return config

    endpoint = normalize_router_endpoint(router)
    config.insert_json5("mode", json.dumps("client"))
    config.insert_json5("connect/endpoints", json.dumps([endpoint]))
    config.insert_json5("listen/endpoints", json.dumps([]))
    config.insert_json5("scouting/multicast/enabled", "false")
    config.insert_json5("scouting/gossip/enabled", "false")
    return config


def open_zenoh_session(router: str | None) -> Any:
    try:
        import zenoh
    except ModuleNotFoundError as error:
        msg = (
            "The Python 'zenoh' package is required to publish. "
            "Install the workshop dependencies or run "
            "`python3 -m pip install eclipse-zenoh`."
        )
        raise RuntimeError(msg) from error

    return zenoh.open(build_zenoh_config(zenoh, router))


def close_if_available(value: object, method_name: str) -> None:
    method = getattr(value, method_name, None)
    if callable(method):
        method()


def publish_animation(
    animation: Animation,
    *,
    key: str,
    router: str | None,
    fps: float,
    repeat: int,
    start_delay: float,
) -> None:
    session = open_zenoh_session(router)
    publisher = cast(Any, session).declare_publisher(key)
    interval = 1.0 / fps
    endpoint = None if router is None else normalize_router_endpoint(router)

    try:
        if start_delay > 0.0:
            time.sleep(start_delay)

        if endpoint is not None:
            print(f"Connecting to Zenoh router at {endpoint}")
        print(
            f"Publishing {animation.positions.shape[0]} frames from "
            f"{animation.path} to '{key}' at {fps:g} Hz"
        )
        publish_repeated_frames(
            publisher,
            animation.positions,
            interval,
            repeat,
        )
    finally:
        close_if_available(publisher, "undeclare")
        close_if_available(session, "close")


def publish_repeated_frames(
    publisher: Any,
    positions: NDArray[np.float32],
    interval: float,
    repeat: int,
) -> None:
    next_publish = time.monotonic()
    repetition = 0

    while repeat == 0 or repetition < repeat:
        for frame in positions:
            publisher.put(serialize_arm_joints(frame))
            next_publish += interval

            sleep_duration = next_publish - time.monotonic()
            if sleep_duration > 0.0:
                time.sleep(sleep_duration)
            else:
                next_publish = time.monotonic()

        repetition += 1


def dry_run(
    animation: Animation,
    fps: float,
    repeat: int,
    key: str,
    router: str | None,
) -> None:
    first_payload = serialize_arm_joints(animation.positions[0])
    endpoint = None if router is None else normalize_router_endpoint(router)
    total_frames: str | int
    if repeat == 0:
        total_frames = "infinite"
    else:
        total_frames = animation.positions.shape[0] * repeat

    print(f"Animation: {animation.path}")
    print(f"Frames: {animation.positions.shape[0]}")
    print(f"FPS: {fps:g}")
    print(f"Zenoh key: {key}")
    if endpoint is not None:
        print(f"Zenoh router: {endpoint}")
    print(f"Total frames to publish: {total_frames}")
    print(f"CDR payload size: {len(first_payload)} bytes")
    print(f"First payload prefix: {first_payload[:8].hex(' ')}")


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    animation = load_animation(args.animation)
    fps = args.fps if args.fps is not None else animation.fps
    repeat = 0 if args.loop else args.repeat

    if args.dry_run:
        dry_run(animation, fps, repeat, args.key, args.router)
        return 0

    try:
        publish_animation(
            animation,
            key=args.key,
            router=args.router,
            fps=fps,
            repeat=repeat,
            start_delay=args.start_delay,
        )
    except KeyboardInterrupt:
        return 130

    return 0


if __name__ == "__main__":
    sys.exit(main())
