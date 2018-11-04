#!/bin/bash

# Get base directory for better referencing
BASEDIR=`cd $(dirname $0); pwd -P`

sh ${BASEDIR}/eigen/tools/install.sh
sh ${BASEDIR}/hulksDatatypes/tools/install.sh
