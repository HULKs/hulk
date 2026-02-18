#!/usr/bin/env sh

MUJOCO_GL=egl uv run train \
    Mjlab-Velocity-Rough-K1 \
    --gpu-ids all \
    --env.scene.num-envs 10000 \
    --enable-nan-guard True
