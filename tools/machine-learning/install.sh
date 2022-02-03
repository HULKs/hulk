#!/usr/bin/env bash

set -e

# change to this directory
cd "$(dirname "$0")"

if [ -n "${VIRTUAL_ENV}" ]; then
  CMAKE_INSTALL_PREFIX=-DCMAKE_INSTALL_PREFIX="${VIRTUAL_ENV}"
else
  read -p "Warning: No virtual environment active. Continue installing system-wide? [y/N] " answer
  case ${answer:0:1} in
    y|Y )
        echo "Installing system-wide..."
    ;;
    * )
        exit 1
    ;;
  esac
fi

set -x

# setup vcpkg
./setup-vcpkg.sh

cd ball_detection

# install C++ tools
rm -Rf build
mkdir -p build
cd build
cmake \
  -G Ninja \
  -DCMAKE_BUILD_TYPE="RelWithDebInfo" \
  ${CMAKE_INSTALL_PREFIX} \
  -DCMAKE_TOOLCHAIN_FILE="../../vcpkg/scripts/buildsystems/vcpkg.cmake" \
  ..
cmake --build . --target install
cd ..

# install Python tools
pip install ./


cd ..

# build predicter
./build-predicter.sh
