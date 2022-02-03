#!/usr/bin/env bash

# runs a docker container with installed tensorflow with /bin/bash
# passthrough all GPUs and set TF_FORCE_GPU_ALLOW_GROWTH to true
# mounts home directory of current user into same location inside the container
# mounts /etc/passwd and /etc/group into the container (for better user environment)
# sets the current working directory
# uses permissions of current user
docker run \
    --gpus all \
    --rm \
    --interactive \
    --tty \
    --volume "$(getent passwd $(id -u) | cut -f6 -d:)":"$(getent passwd $(id -u) | cut -f6 -d:)" \
    --volume "/etc/passwd":"/etc/passwd":ro \
    --volume "/etc/group":"/etc/group":ro \
    --workdir "$(pwd)" \
    --user "$(id -u):$(id -g)" \
    --env "TF_FORCE_GPU_ALLOW_GROWTH=true" \
    "hulks-evolver" \
    $@
