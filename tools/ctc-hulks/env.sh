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

export PATH=${BASEDIR}/x-tools/i686-nao-linux-gnu/bin:${BASEDIR}/tools/ct-ng/bin:$PATH

