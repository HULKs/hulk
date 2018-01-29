#!/bin/bash
#keep this script at root DIR of OFA (tools/ofa/startOfa.sh)
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$DIR/src/ofa"
nodejs index.js
