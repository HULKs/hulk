#!/usr/bin/env bash

set -e

if [ -z $1 ]; then
  echo "Usage: ${0} DVC_FILE"
  exit 1
fi

echo "Determining size of ${1}..."

CACHE_HASH_PREFIX=$(grep md5 "${1}" | cut -c8-9)
CACHE_HASH_REMAINDER=$(grep md5 "${1}" | cut -c10-)
# CACHE_HASH_REMAINDER has .dir suffix -> is directory, else is single file

if [[ "${CACHE_HASH_REMAINDER}" =~ \.dir$ ]]; then
  ssh -p 210 hulk@rkost.org "cat /home/hulk/data/${CACHE_HASH_PREFIX}/${CACHE_HASH_REMAINDER} | jq -r '.[].md5' | while read hash; do echo /home/hulk/data/\$(echo \${hash} | cut -c1-2)/\$(echo \${hash} | cut -c3-); done | tr '\n' '\0' | du -ch --files0-from=- | tail -n 1"
else
  ssh -p 210 hulk@rkost.org "du -ch /home/hulk/data/${CACHE_HASH_PREFIX}/${CACHE_HASH_REMAINDER} | tail -n 1"
fi
