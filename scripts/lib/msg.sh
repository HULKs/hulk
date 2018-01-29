#!/bin/bash

# For pretty printing
# -e: error
# -s: section
# -n: normal
# -w: warning
function msg {
  if [ "$#" -eq 2 ]; then
    case "$1" in
      "-e" )
        printf "\e[1;31m-- $2\e[0m\n\n"
        ;;
      "-s" )
        printf "\e[1;32m-- $2\e[0m\n\n"
        ;;
      "-w" )
        printf "\e[1;33m-- $2\e[0m\n\n"
        ;;
      "-n" )
        printf "\e[1;38m-- $2\e[0m\n\n"
        ;;
    esac
  elif [ "$#" -eq 1 ]; then
    printf "$1"
  fi
}
