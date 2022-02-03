#!/usr/bin/env bash

set -e -x

# change to this directory
cd "$(dirname "$0")"

# build predicter
cd predicter
rm -Rf build
mkdir -p build
cd build
cmake \
  -G Ninja \
  -DCMAKE_BUILD_TYPE="RelWithDebInfo" \
  -DCMAKE_TOOLCHAIN_FILE="../../vcpkg/scripts/buildsystems/vcpkg.cmake" \
  ..
cmake --build .
cd ..
