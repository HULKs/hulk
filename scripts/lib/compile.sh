#!/bin/bash

source "${BASEDIR}/scripts/lib/linkBuild.sh"

function njobs {
  if [ -f "${BASEDIR}/.njobs" ]; then
    cat "${BASEDIR}/.njobs"
    return
  fi
  if [[ `uname` == "Darwin" ]]; then
    echo $((`gnproc` + 1))
  else
    echo $((`nproc` + 1))
  fi
}

function compile {
  if [ "$#" -ne 5 ]; then
    return 1
  fi
  local BASEDIR="$1"
  local TARGET="$2"
  local BUILD_TYPE="$3"
  local VERBOSE=$4
  local JOBS=$5
  local DIR="${BASEDIR}/build/${TARGET}/${BUILD_TYPE}"
  # check if there is something generated
  if [ ! -f "${DIR}/CMakeCache.txt" ]; then
    return 1
  fi
  # save the current target and build type
  echo "${TARGET}" > "${BASEDIR}/.current.tc"
  echo "${BUILD_TYPE}" > "${BASEDIR}/.current.bt"
  # compile (currently done with make, but cmake --build is an option)
  cd "${DIR}"
  if ${VERBOSE}; then
    make -j${JOBS} VERBOSE=1
  else
    make -j${JOBS}
  fi
  local RESULT="$?"
  return "${RESULT}"
}
