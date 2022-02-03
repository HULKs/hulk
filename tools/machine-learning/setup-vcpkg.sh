#!/usr/bin/env bash

set -e -x

# Get base directory for better referencing
BASE_DIRECTORY="$(cd $(dirname $0); pwd)"
VERSION_TAG="2020.06"

if [ -z "$VCPKG_DIR" ]; then
  VCPKG_DIR="${BASE_DIRECTORY}/vcpkg"
fi

# Either clone vcpkg or fetch existing repos's remote
if [ ! -e "${VCPKG_DIR}" ]; then
  git clone -b "${VERSION_TAG}" git@github.com:microsoft/vcpkg.git ${VCPKG_DIR}
  cd ${VCPKG_DIR}
else
  cd ${VCPKG_DIR}
  git fetch --all
  git switch --detach "${VERSION_TAG}"
fi

# Build vcpkg
if [ ! -e "${BASE_DIRECTORY}/vcpkg/vcpkg" ]; then
  ./bootstrap-vcpkg.sh -disableMetrics
fi

# Install packages
./vcpkg install --recurse --overlay-ports="${BASE_DIRECTORY}/vcpkg-ports" stb compilednn nlohmann-json protobuf tbb cxxopts
