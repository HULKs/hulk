#!/bin/sh

xhost +local: \

sudo podman network create --ignore gamecontroller-bridge

sudo podman run -it --rm \
 --network=gamecontroller-bridge \
 --ipc=host \
 --env DISPLAY \
 --env XDG_RUNTIME_DIR=/tmp \
 --env WEBKIT_DISABLE_COMPOSITING_MODE=1 \
 --volume /tmp/.X11-unix:/tmp/.X11-unix:ro \
 --user root \
 --security-opt label=disable \
 gamecontroller:latest