#!/bin/bash

function numberToIP {
  if [ "$#" -ne 1 ]; then
    return 1
  fi
  local NUMBER="$1"
  if [[ "${NUMBER}" =~ ^[0-9]*$ ]]; then
    echo "10.1.YOUR_TEAM_NUMBER_HERE.$((10 + ${NUMBER}))"
  elif [[ "${NUMBER}" =~ ^[0-9]*w$ ]]; then
    echo "10.0.YOUR_TEAM_NUMBER_HERE.$((10 + ${NUMBER::-1}))"
  else
    echo "${NUMBER}"
  fi
  return 0
}
