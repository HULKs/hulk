#!/bin/bash

HOST_TEGRA_LIB_PATH="/usr/lib/aarch64-linux-gnu/tegra"
DEVICES="--device nvidia.com/gpu=all"

sudo podman run -it --rm \
  --privileged \
  --network=host \
  $DEVICES \
  -v ${HOST_TEGRA_LIB_PATH}:${HOST_TEGRA_LIB_PATH} \
  -v /home/booster/hulk:/home/booster/hulk \
  -v /home/booster/.cache:/home/booster/.cache \
  -v /home/booster/.cargo:/root/.cargo \
  --env LD_LIBRARY_PATH=${HOST_TEGRA_LIB_PATH}:$LD_LIBRARY_PATH \
  --env ORT_DYLIB_PATH=/usr/local/lib/libonnxruntime.so \
  rust-trt-inference:latest \
  bash -c "cd /home/booster/hulk && ./bin/hulk"
