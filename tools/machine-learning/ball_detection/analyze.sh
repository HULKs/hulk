#!/usr/bin/env bash

# change to this directory
cd "$(dirname "$0")"

parallel -j 4 clang-tidy --quiet -p build ::: $(find grid_cropper predicter runner transformer -name \*.cpp -or -name \*.hpp)
cppcheck --quiet grid_cropper predicter runner transformer
# iwyu-tool -p build
