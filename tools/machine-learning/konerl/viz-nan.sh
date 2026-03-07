#!/usr/bin/env sh

mkdir -p /tmp/mjlab/nan_dumps
rsync -a ole@jarvis.es.tuhh.de:/tmp/mjlab/nan_dumps /tmp/mjlab/
uv run viz-nan /tmp/mjlab/nan_dumps/nan_dump_latest.npz
