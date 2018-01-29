#!/bin/bash

DOCKER_IMAGE_NAME="bighulk:5000/build"

function iAmInDocker {
  [ -f /.dockerenv ]
}

function handleDocker {
  local BASEDIR="$1"
  local USE_DOCKER=false
  shift
  if [ -f "${BASEDIR}/toolchain/docker" ] && ! iAmInDocker; then
    USE_DOCKER=true
  fi
  if ${USE_DOCKER}; then
    docker run -it --net host --rm -u $UID:$GID -v "${BASEDIR}:/nao" --entrypoint /nao/scripts/`basename "$0"` "${DOCKER_IMAGE_NAME}" "$@"
  else
    run "$@"
  fi
}
