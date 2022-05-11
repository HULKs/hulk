#!/usr/bin/env bash

set -e -x
cd "$(dirname "$0")"

RECHENKNECHT_DESTINATION=hulk@rechenknecht
DOCKER_VM_DESTINATION=hulk@

RECHENKNECHT_PATH=/srv/v2
DOCKER_VM_PATH=/home/hulk/ci/github-runners/v2

update() {
  ssh -t ${1} bash -c "set -x && cd ${2} && docker-compose down"
  scp docker-compose.yml Dockerfile ${1}:${2}
  ssh ${1} bash -c "set -x && cd ${2} && docker-compose build --pull && docker-compose up -d"
}

update ${RECHENKNECHT_DESTINATION} ${RECHENKNECHT_PATH}
update ${DOCKER_VM_DESTINATION} ${DOCKER_VM_PATH}
