#!/bin/bash

function continue_install {
  echo -n "$1 Continue? [y/n] "
  read -t 10 OPT
  case $OPT in
    n | N)
      exit;;
  esac
}

if [ -n "$ZSH_VERSION" ]; then
  export BASEDIR=`cd $(dirname ${(%):-%x}); pwd -P`
elif [ -n "$BASH_VERSION" ]; then
  export BASEDIR=`cd $(dirname $BASH_SOURCE); pwd -P`
else
  echo "No compatible shell!"
fi

source $BASEDIR/versions

# This is a small thing to ensure ct-ng sanity checks arent obstructed.
unset LD_LIBRARY_PATH
unset CPATH

export PATH=${BASEDIR}/x-tools/i686-nao-linux-gnu/bin:${BASEDIR}/tools/ct-ng/bin:$PATH

