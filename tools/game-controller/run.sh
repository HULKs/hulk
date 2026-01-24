#!/bin/sh

xhost +local: \

sudo podman network create --ignore gamecontroller-bridge

sudo podman run -it --rm \
 --network=gamecontroller-bridge \
 --ipc=host \
 --device /dev/dri \
 --device /dev/input \
 --device /dev/bus/usb \
 --env DISPLAY \
 --env XDG_RUNTIME_DIR=/tmp \
 --env WEBKIT_DISABLE_COMPOSITING_MODE=1 \
 --env LIBGL_ALWAYS_SOFTWARE=1 \
 -e RUST_BACKTRACE=full \
 --volume /tmp/.X11-unix:/tmp/.X11-unix:ro \
 --user root \
 --security-opt label=disable \
 gamecontroller:latest