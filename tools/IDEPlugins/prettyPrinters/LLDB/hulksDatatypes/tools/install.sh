#!/bin/bash

# Get base directory for better referencing
BASEDIR=`cd $(dirname $0); pwd -P`
BASEDIR=${BASEDIR%/*}
INSTALL_PATH=${BASEDIR}

# color helpers
function heading () {
	printf '\e[48;1;4m%s\e[0m \n' "$1"
}

function notice () {
	printf '\e[0;32m%s\e[0m \n' "$1"
}

function error () {
	printf '\e[41m%s\e[0m \n' "$1"
	exit;
}

function warn () {
	printf '\e[48;5;208m%s\e[0m \n' "$1"
}

#
# Add to lldbinit
#
grep -Fq "LLDB_HULKs_type_integration.py" ~/.lldbinit
ALREADY_INSTALLED=$?
if [ ! ${ALREADY_INSTALLED} -eq 0 ]; then
	echo 'command script import "'${INSTALL_PATH}'/LLDB_HULKs_type_integration.py"' >> ~/.lldbinit
	notice "Adding HULKs pretty printer to ~/.lldbinit"
else
	warn "Skipping HULKs pretty printer, already installed."
fi
