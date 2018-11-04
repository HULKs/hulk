#!/bin/bash

function linkCurrentBuild {
  if [ "$#" -ne 3 ]; then
    return 1
  fi
  local BASEDIR="$1"
  local TARGET="$2"
  local BUILD_TYPE="$3"

  if [ ! -d "${BASEDIR}/build/${TARGET}/${BUILD_TYPE}" ]; then
    return 2
  fi

  ln -snf "${BUILD_TYPE}" "${BASEDIR}/build/${TARGET}/current"
}
