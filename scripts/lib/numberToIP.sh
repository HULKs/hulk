#!/bin/bash

TEAM_NUMBER="xx"

function numberToIP {
  if [ "$#" -ne 1 ]; then
    return 1
  fi
  local NUMBER="$1"
  if [[ "${NUMBER}" =~ ^[0-9]*$ ]]; then
    echo "10.1.${TEAM_NUMBER}.$((10 + ${NUMBER}))"
  elif [[ "${NUMBER}" =~ ^[0-9]*w$ ]]; then
    echo "10.0.${TEAM_NUMBER}.$((10 + ${NUMBER::-1}))"
  else
    echo "${NUMBER}"
  fi
  return 0
}
