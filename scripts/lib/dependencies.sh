#!/bin/bash

function assertDependencies {
  for TOOL in "cmake" "make" "rsync" "ssh" "curl"; do
    hash "${TOOL}" 2>/dev/null || { echo >&2 "${TOOL} is not installed. Consult https://github.com/HULKs/nao/wiki/System-Setup for finding information on how to install things."; exit 1; }
    shift
  done
}

function assertDependenciesInstallScript {
  for TOOL in "nc" "ssh" "sshpass"; do
    hash "${TOOL}" 2>/dev/null || { echo >&2 "${TOOL} is not installed. Consult https://github.com/HULKs/nao/wiki/System-Setup for finding information on how to install things."; exit 1; }
    shift
  done
}
