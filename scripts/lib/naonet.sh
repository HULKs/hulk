#!/bin/bash

function naocmd {
  if [ "$#" -ne 3 ]; then
    return 1
  fi
  local BASEDIR="$1"
  local NAME="$2"
  local COMMAND="$3"
  ssh -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o LogLevel=quiet -o ConnectTimeout=5 -l nao -i "${BASEDIR}/scripts/ssh_key" -t "${NAME}" "${COMMAND}"
}

function naossh {
  if [ "$#" -ne 2 ]; then
    return 1
  fi
  local BASEDIR="$1"
  local NAME="$2"
  ssh -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o LogLevel=quiet -l nao -i "${BASEDIR}/scripts/ssh_key" "${NAME}"
}

function naocp {
  if [ "$#" -lt 3 ]; then
    return 1
  fi
  local BASEDIR="$1"
  # take all arguments from second to penultimate one as source
  local SRC=${@:2:$(expr $# - 2)}
  # take last argument as destination
  local DST=${@:$#}
  scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o LogLevel=quiet -i "${BASEDIR}/scripts/ssh_key" -r ${SRC} "${DST}"
}

function naocmdpass {
  if [ "$#" -ne 2 ]; then
    return 1
  fi
  local NAME="$1"
  local COMMAND="$2"
  sshpass -p nao ssh -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o LogLevel=quiet -l nao -t "${NAME}" "${COMMAND}"
}

function naocppass {
  if [ "$#" -ne 2 ]; then
    return 1
  fi
  local SRC="$1"
  local DST="$2"
  sshpass -p nao scp -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o LogLevel=quiet -r "${SRC}" "${DST}"
}
